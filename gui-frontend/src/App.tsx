import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { check, type DownloadEvent, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { useTheme } from "./theme";
import { useManagers } from "./hooks/useManagers";
import { usePackages } from "./hooks/usePackages";
import { useTasks } from "./hooks/useTasks";
import { useAppStore } from "./store/appStore";
import { useManagerStore } from "./store/managerStore";
import { usePackageStore } from "./store/packageStore";
import { useTaskStore } from "./store/taskStore";
import { useI18n } from "./lib/i18n";
import {
  getAppLogPath,
  getAppLogs,
  getPackageInfo,
  isTauri,
  openExternalUrl,
  searchPackages,
  uninstallPackage,
  updateOutdatedPackages,
  updatePackage
} from "./lib/api";
import type { Job, ManagerStatus, Package } from "./types";

const NAV_ITEMS = [
  { id: "dashboard", labelKey: "nav.dashboard" },
  { id: "manager", labelKey: "nav.manager" },
  { id: "tasks", labelKey: "nav.tasks" },
  { id: "settings", labelKey: "nav.settings" }
] as const;

const PACKAGE_INFO_TTL_MS = 5 * 60 * 1000;
const PACKAGE_INFO_PREFETCH_LIMIT = 20;
const PACKAGE_INFO_PREFETCH_CONCURRENCY = 3;

const formatSize = (bytes: number): string => {
  if (!Number.isFinite(bytes) || bytes <= 0) {
    return "0 B";
  }
  const units = ["B", "KB", "MB", "GB"];
  let size = bytes;
  let unitIndex = 0;
  while (size >= 1024 && unitIndex < units.length - 1) {
    size /= 1024;
    unitIndex += 1;
  }
  return `${size.toFixed(size >= 10 ? 0 : 1)} ${units[unitIndex]}`;
};

const isGitLink = (url: string): boolean => {
  const lower = url.toLowerCase();
  return (
    lower.includes("github.com") ||
    lower.includes("gitlab.com") ||
    lower.includes("gitee.com")
  );
};

export const App: React.FC = () => {
  const { theme, setTheme } = useTheme();
  const {
    sidebarCollapsed,
    currentView,
    setCurrentView,
    searchQuery,
    setSearchQuery,
    setSidebarCollapsed
  } = useAppStore();
  const { t } = useI18n();
  const { managers, loadManagers, refreshManager, refreshAll } = useManagers();
  const {
    selectedManager,
    selectManager,
    packageScope,
    packageDirectory,
    setPackageScope,
    setPackageDirectory
  } = useManagerStore();
  const {
    packages,
    loading: packagesLoading,
    loadPackages
  } = usePackages(selectedManager, packageScope, packageDirectory);
  const { filter, setFilter, selectedPackage, selectPackage } = usePackageStore();
  const { tasks, currentTask, cancelTask, removeTask, clearTasks } = useTasks();
  const { loadTasks, updateTask, addTask } = useTaskStore();
  const [searching, setSearching] = useState(false);
  const [searchResults, setSearchResults] = useState<Package[] | null>(null);
  const [searchError, setSearchError] = useState<string | null>(null);
  const [batchUpdating, setBatchUpdating] = useState(false);
  const [batchUpdateMessage, setBatchUpdateMessage] = useState<string | null>(null);
  const [packageActionMessage, setPackageActionMessage] = useState<string | null>(null);
  const [packageActionKey, setPackageActionKey] = useState<string | null>(null);
  const [confirmUninstallPackage, setConfirmUninstallPackage] =
    useState<Package | null>(null);
  const [updateDialog, setUpdateDialog] = useState<{
    mode: "info" | "confirm";
    title: string;
    message: string;
    action?: "single" | "batch";
    pkg?: Package;
  } | null>(null);
  const [cancelingTaskId, setCancelingTaskId] = useState<string | null>(null);
  const [selectedPackageInfo, setSelectedPackageInfo] = useState<Package | null>(null);
  const [packageInfoLoading, setPackageInfoLoading] = useState(false);
  const [packageInfoError, setPackageInfoError] = useState<string | null>(null);
  const [refreshMessage, setRefreshMessage] = useState<string | null>(null);
  const [logsOpen, setLogsOpen] = useState(false);
  const [logsLoading, setLogsLoading] = useState(false);
  const [logsError, setLogsError] = useState<string | null>(null);
  const [logsLines, setLogsLines] = useState<string[]>([]);
  const [logPath, setLogPath] = useState<string>("");
  const [logFilter, setLogFilter] = useState("");
  const [logLevelFilter, setLogLevelFilter] = useState<string>("all");
  const searchTimer = useRef<number | null>(null);
  const searchInputRef = useRef<HTMLInputElement | null>(null);
  const batchMessageTimer = useRef<number | null>(null);
  const packageMessageTimer = useRef<number | null>(null);
  const refreshMessageTimer = useRef<number | null>(null);
  const packageInfoRequestId = useRef(0);
  const packageInfoCache = useRef(
    new Map<string, { info: Package; cachedAt: number }>()
  );
  const packageInfoPrefetchRequestId = useRef(0);
  const homepageLink = selectedPackageInfo?.homepage?.trim() ?? "";
  const repositoryLink = selectedPackageInfo?.repository?.trim() ?? "";
  const gitLink =
    repositoryLink || (homepageLink && isGitLink(homepageLink) ? homepageLink : "");
  const showHomepage =
    homepageLink.length > 0 &&
    !isGitLink(homepageLink) &&
    (!repositoryLink || homepageLink !== repositoryLink);
  const showGit = gitLink.length > 0;

  const showBatchMessage = useCallback((message: string) => {
    setRefreshMessage(null);
    setBatchUpdateMessage(message);
    if (batchMessageTimer.current) {
      window.clearTimeout(batchMessageTimer.current);
    }
    batchMessageTimer.current = window.setTimeout(() => {
      setBatchUpdateMessage(null);
    }, 3000);
  }, []);

  const showPackageMessage = useCallback((message: string) => {
    setPackageActionMessage(message);
    if (packageMessageTimer.current) {
      window.clearTimeout(packageMessageTimer.current);
    }
    packageMessageTimer.current = window.setTimeout(() => {
      setPackageActionMessage(null);
    }, 3000);
  }, []);

  const handleExternalLink = useCallback(
    async (event: React.MouseEvent<HTMLAnchorElement>, url: string) => {
      event.preventDefault();
      try {
        await openExternalUrl(url);
      } catch (error) {
        console.error("Open external url failed:", error);
        showPackageMessage("打开链接失败，请检查日志");
      }
    },
    [showPackageMessage]
  );

  const showRefreshMessage = useCallback((message: string) => {
    setBatchUpdateMessage(null);
    setRefreshMessage(message);
    if (refreshMessageTimer.current) {
      window.clearTimeout(refreshMessageTimer.current);
    }
    refreshMessageTimer.current = window.setTimeout(() => {
      setRefreshMessage(null);
    }, 3000);
  }, []);

  const runSearch = useCallback(async (query: string) => {
    const trimmedQuery = query.trim();
    if (!trimmedQuery) {
      setSearchResults(null);
      setSearchError(null);
      return;
    }

    setSearching(true);
    setSearchError(null);
    try {
      const results = await searchPackages(trimmedQuery);
      setSearchResults(results);
    } catch (error) {
      setSearchError(String(error));
      setSearchResults([]);
    } finally {
      setSearching(false);
    }
  }, []);

  useEffect(() => {
    return () => {
      if (batchMessageTimer.current) {
        window.clearTimeout(batchMessageTimer.current);
      }
      if (packageMessageTimer.current) {
        window.clearTimeout(packageMessageTimer.current);
      }
      if (refreshMessageTimer.current) {
        window.clearTimeout(refreshMessageTimer.current);
      }
    };
  }, []);

  useEffect(() => {
    setSearchResults(null);
    setSearchError(null);
  }, [selectedManager]);

  useEffect(() => {
    selectPackage(null);
  }, [packageScope, packageDirectory, selectPackage]);

  useEffect(() => {
    if (!selectedPackage) {
      setSelectedPackageInfo(null);
      setPackageInfoError(null);
      setPackageInfoLoading(false);
      return;
    }

    const requestId = packageInfoRequestId.current + 1;
    packageInfoRequestId.current = requestId;
    const cacheKey = `${packageScope}|${packageDirectory}|${selectedPackage.manager}|${selectedPackage.name}`;
    const cached = packageInfoCache.current.get(cacheKey);
    if (cached && Date.now() - cached.cachedAt < PACKAGE_INFO_TTL_MS) {
      setSelectedPackageInfo(cached.info);
      setPackageInfoLoading(false);
      setPackageInfoError(null);
      return;
    }
    setPackageInfoLoading(true);
    setPackageInfoError(null);

    getPackageInfo(
      selectedPackage.manager,
      selectedPackage.name,
      packageScope,
      packageDirectory
    )
      .then((info) => {
        if (packageInfoRequestId.current !== requestId) {
          return;
        }
        packageInfoCache.current.set(cacheKey, { info, cachedAt: Date.now() });
        setSelectedPackageInfo(info);
      })
      .catch((error) => {
        if (packageInfoRequestId.current !== requestId) {
          return;
        }
        setPackageInfoError(String(error));
        setSelectedPackageInfo(null);
      })
      .finally(() => {
        if (packageInfoRequestId.current !== requestId) {
          return;
        }
        setPackageInfoLoading(false);
      });
  }, [selectedPackage, packageScope, packageDirectory]);

  const prefetchPackageInfos = useCallback(
    (items: Package[]) => {
      const requestId = packageInfoPrefetchRequestId.current + 1;
      packageInfoPrefetchRequestId.current = requestId;
      let cursor = 0;
      const runNext = async (): Promise<void> => {
        if (packageInfoPrefetchRequestId.current !== requestId) {
          return;
        }
        const next = items[cursor];
        cursor += 1;
        if (!next) {
          return;
        }
        const cacheKey = `${packageScope}|${packageDirectory}|${next.manager}|${next.name}`;
        const cached = packageInfoCache.current.get(cacheKey);
        if (cached && Date.now() - cached.cachedAt < PACKAGE_INFO_TTL_MS) {
          await runNext();
          return;
        }
        try {
          const info = await getPackageInfo(
            next.manager,
            next.name,
            packageScope,
            packageDirectory
          );
          if (packageInfoPrefetchRequestId.current !== requestId) {
            return;
          }
          packageInfoCache.current.set(cacheKey, { info, cachedAt: Date.now() });
        } catch (error) {
          console.warn("预加载包信息失败:", error);
        } finally {
          await runNext();
        }
      };

      const workers = Array.from(
        { length: PACKAGE_INFO_PREFETCH_CONCURRENCY },
        () => runNext()
      );
      void Promise.all(workers);
    },
    [packageScope, packageDirectory]
  );

  useEffect(() => {
    if (packagesLoading || packages.length === 0 || !selectedManager) {
      return;
    }
    const candidates = packages.slice(0, PACKAGE_INFO_PREFETCH_LIMIT);
    // 后台预热包详情，避免点击时才触发网络请求。
    prefetchPackageInfos(candidates);
  }, [
    packages,
    packagesLoading,
    selectedManager,
    prefetchPackageInfos,
    packageScope,
    packageDirectory
  ]);

  useEffect(() => {
    if (!isTauri()) {
      return;
    }

    let unlistenProgress: (() => void) | null = null;
    let unlistenComplete: (() => void) | null = null;

    Promise.all([
      listen("task-progress", (event) => {
        try {
          const payload = event.payload as { taskId: string; progress: number };
          updateTask(payload.taskId, {
            status: "Running",
            progress: payload.progress
          });
        } catch (error) {
          console.error("Failed to handle task-progress:", error);
        }
      }),
      listen("task-complete", (event) => {
        try {
          const payload = event.payload as {
            id: string;
            status: string;
            manager?: string | null;
          };
          updateTask(payload.id, { status: payload.status as Job["status"] });
          loadTasks();
          // 当任务完成且是当前选中的管理器时，刷新包列表
          if (
            payload.manager &&
            selectedManager &&
            payload.manager === selectedManager &&
            (packageScope !== "local" || packageDirectory.trim().length > 0)
          ) {
            // 对于取消的任务，强制刷新缓存，因为后端不会自动清除
            // 对于成功/失败的任务，后端已清除缓存，使用普通刷新即可
            const shouldForceRefresh = payload.status === "Canceled";
            loadPackages(
              selectedManager,
              packageScope,
              packageDirectory,
              shouldForceRefresh
            );
          }
        } catch (error) {
          console.error("Failed to handle task-complete:", error);
        }
      })
    ])
      .then(([progressUnlisten, completeUnlisten]) => {
        unlistenProgress = progressUnlisten;
        unlistenComplete = completeUnlisten;
      })
      .catch((error) => {
        console.error("Failed to setup event listeners:", error);
      });

    return () => {
      if (unlistenProgress) {
        unlistenProgress();
      }
      if (unlistenComplete) {
        unlistenComplete();
      }
    };
  }, [loadTasks, updateTask, selectedManager, packageScope, packageDirectory, loadPackages]);

  useEffect(() => {
    if (!searchQuery.trim()) {
      setSearching(false);
      setSearchResults(null);
      setSearchError(null);
      if (searchTimer.current) {
        window.clearTimeout(searchTimer.current);
      }
      return;
    }

    if (searchTimer.current) {
      window.clearTimeout(searchTimer.current);
    }

    setSearching(true);
    searchTimer.current = window.setTimeout(() => {
      if (currentView !== "manager") {
        setCurrentView("manager");
      }
      runSearch(searchQuery).catch((error) => {
        console.error("Search failed:", error);
      });
    }, 300);

    return () => {
      if (searchTimer.current) {
        window.clearTimeout(searchTimer.current);
      }
    };
  }, [searchQuery, currentView, setCurrentView, runSearch]);

  const filteredPackages = useMemo(() => {
    if (filter === "outdated") {
      return packages.filter((pkg) => pkg.outdated);
    }
    return packages;
  }, [packages, filter]);

  const groupedSearchResults = useMemo(() => {
    if (!searchResults) {
      return [];
    }
    const map = new Map<string, Package[]>();
    for (const pkg of searchResults) {
      const list = map.get(pkg.manager) ?? [];
      list.push(pkg);
      map.set(pkg.manager, list);
    }
    return Array.from(map.entries())
      .sort(([a], [b]) => a.localeCompare(b))
      .map(([manager, items]) => ({
        manager,
        items: items.sort((left, right) => left.name.localeCompare(right.name))
      }));
  }, [searchResults]);

  const onSearchSubmit = async (event: React.FormEvent) => {
    event.preventDefault();
    if (!searchQuery.trim()) {
      return;
    }
    if (searchTimer.current) {
      window.clearTimeout(searchTimer.current);
    }
    if (currentView !== "manager") {
      setCurrentView("manager");
    }
    try {
      await runSearch(searchQuery);
    } catch (error) {
      console.error("Search failed:", error);
    }
  };

  const executeBatchUpdate = async () => {
    if (!selectedManager || batchUpdating) {
      return;
    }
    if (packageScope === "local" && !packageDirectory.trim()) {
      showBatchMessage("请先填写目录路径");
      return;
    }
    setBatchUpdating(true);
    try {
      const taskId = await updateOutdatedPackages(
        selectedManager,
        packageScope,
        packageDirectory
      );
      addTask({
        id: taskId,
        manager: selectedManager,
        operation: "Update",
        target: "outdated",
        status: "Running",
        progress: 0,
        step: "started",
        started_at: new Date().toISOString(),
        finished_at: null,
        logs: []
      });
      showBatchMessage(`已提交批量更新任务：${taskId}`);
      await loadTasks();
      setCurrentView("tasks");
    } catch (error) {
      console.error("Batch update failed:", error);
      showBatchMessage("批量更新失败，请检查日志");
    } finally {
      setBatchUpdating(false);
    }
  };

  const onRefresh = useCallback(async () => {
    try {
      if (packageScope === "local" && selectedManager && !packageDirectory.trim()) {
        showBatchMessage("请先填写目录路径");
        return;
      }
      if (selectedManager) {
        // 强制刷新包列表（清除后端缓存并重新获取数据）
        await loadPackages(selectedManager, packageScope, packageDirectory, true);
        // 更新管理器统计数据（package_count, outdated_count）
        await loadManagers();
        showBatchMessage(`已刷新 ${selectedManager}`);
      } else {
        await refreshAll();
        showBatchMessage("已刷新所有管理器");
      }
    } catch (error) {
      console.error("Refresh failed:", error);
      showBatchMessage("刷新失败，请检查日志");
    }
  }, [
    selectedManager,
    loadManagers,
    refreshManager,
    refreshAll,
    loadPackages,
    showBatchMessage,
    packageScope,
    packageDirectory
  ]);

  const onRefreshAllWithMessage = useCallback(async () => {
    try {
      await refreshAll();
      showRefreshMessage("已刷新所有管理器");
    } catch (error) {
      console.error("Refresh all failed:", error);
      showRefreshMessage("刷新失败，请检查日志");
    }
  }, [refreshAll, showRefreshMessage]);

  const openLogs = useCallback(async () => {
    setLogsOpen(true);
    setLogsLoading(true);
    setLogsError(null);
    try {
      const [lines, path] = await Promise.all([getAppLogs(), getAppLogPath()]);
      setLogsLines(lines);
      setLogPath(path);
    } catch (error) {
      setLogsError(String(error));
      setLogsLines([]);
      setLogPath("");
    } finally {
      setLogsLoading(false);
    }
  }, []);

  const displayLogs = useMemo(() => {
    return logsLines
      .map((line) => {
      try {
        const parsed = JSON.parse(line) as {
          timestamp?: string;
          level?: string;
          message?: string;
          target?: string;
        };
        const time = parsed.timestamp ?? "";
        const level = parsed.level ? parsed.level.toUpperCase() : "INFO";
        const message = parsed.message ?? line;
        const target = parsed.target ? ` ${parsed.target}` : "";
        return {
          raw: line,
          text: `${time} [${level}]${target} ${message}`.trim(),
          level,
        };
      } catch {
        return { raw: line, text: line, level: "INFO" };
      }
    })
      .filter((entry) => {
        if (logLevelFilter !== "all" && entry.level !== logLevelFilter) {
          return false;
        }
        if (!logFilter.trim()) {
          return true;
        }
        const needle = logFilter.trim().toLowerCase();
        return entry.text.toLowerCase().includes(needle);
      });
  }, [logsLines, logFilter, logLevelFilter]);

  const onUpdateSelectedPackage = useCallback(
    async (pkg: Package) => {
      if (packageActionKey) {
        return;
      }
      if (!pkg.outdated) {
        setUpdateDialog({
          mode: "info",
          title: "无需更新",
          message: "当前已是最新版本。"
        });
        return;
      }
      setUpdateDialog({
        mode: "confirm",
        title: "确认更新",
        message: `确认更新 ${pkg.name} 吗？`,
        action: "single",
        pkg
      });
    },
    [packageActionKey]
  );

  const executeUpdatePackage = useCallback(
    async (pkg: Package) => {
      const actionKey = `${pkg.manager}:${pkg.name}:update`;
      if (packageActionKey) {
        return;
      }
      setPackageActionKey(actionKey);
      try {
        const taskId = await updatePackage(
          pkg.manager,
          pkg.name,
          packageScope,
          packageDirectory
        );
        addTask({
          id: taskId,
          manager: pkg.manager,
          operation: "Update",
          target: pkg.name,
          status: "Running",
          progress: 0,
          step: "started",
          started_at: new Date().toISOString(),
          finished_at: null,
          logs: []
        });
        showPackageMessage(`已提交更新任务：${taskId}`);
        await loadTasks();
        setCurrentView("tasks");
      } catch (error) {
        console.error("Update package failed:", error);
        showPackageMessage("更新失败，请检查日志");
      } finally {
        setPackageActionKey(null);
      }
    },
    [
      packageActionKey,
      loadTasks,
      setCurrentView,
      showPackageMessage,
      addTask,
      packageScope,
      packageDirectory
    ]
  );

  const onBatchUpdateRequest = useCallback(
    (hasOutdated: boolean) => {
      if (batchUpdating || !selectedManager) {
        return;
      }
      if (!hasOutdated) {
        setUpdateDialog({
          mode: "info",
          title: "无需更新",
          message: "当前已是最新版本。"
        });
        return;
      }
      setUpdateDialog({
        mode: "confirm",
        title: "确认更新",
        message: `确认更新 ${selectedManager} 的可更新包吗？`,
        action: "batch"
      });
    },
    [batchUpdating, selectedManager]
  );

  const onUninstallSelectedPackage = useCallback(
    async (pkg: Package) => {
      const actionKey = `${pkg.manager}:${pkg.name}:uninstall`;
      if (packageActionKey) {
        return;
      }
      setConfirmUninstallPackage(pkg);
    },
    [packageActionKey]
  );

  const executeUninstallPackage = useCallback(
    async (pkg: Package) => {
      const actionKey = `${pkg.manager}:${pkg.name}:uninstall`;
      if (packageActionKey) {
        return;
      }
      setPackageActionKey(actionKey);
      try {
        const taskId = await uninstallPackage(
          pkg.manager,
          pkg.name,
          false,
          packageScope,
          packageDirectory
        );
        addTask({
          id: taskId,
          manager: pkg.manager,
          operation: "Uninstall",
          target: pkg.name,
          status: "Running",
          progress: 0,
          step: "started",
          started_at: new Date().toISOString(),
          finished_at: null,
          logs: []
        });
        showPackageMessage(`已提交卸载任务：${taskId}`);
        await loadTasks();
        setCurrentView("tasks");
      } catch (error) {
        console.error("Uninstall package failed:", error);
        showPackageMessage("卸载失败，请检查日志");
      } finally {
        setPackageActionKey(null);
      }
    },
    [
      packageActionKey,
      loadTasks,
      setCurrentView,
      showPackageMessage,
      addTask,
      packageScope,
      packageDirectory
    ]
  );

  useEffect(() => {
    const isEditableTarget = (target: EventTarget | null) => {
      if (!(target instanceof HTMLElement)) {
        return false;
      }
      const tag = target.tagName;
      return (
        target.isContentEditable ||
        tag === "INPUT" ||
        tag === "TEXTAREA" ||
        tag === "SELECT"
      );
    };

    const handleShortcut = (event: KeyboardEvent) => {
      if (event.defaultPrevented) {
        return;
      }
      const key = event.key.toLowerCase();
      const metaPressed = event.metaKey || event.ctrlKey;

      if (metaPressed && key === "k") {
        event.preventDefault();
        if (currentView !== "manager") {
          setCurrentView("manager");
        }
        searchInputRef.current?.focus();
        searchInputRef.current?.select();
        return;
      }

      if (metaPressed && key === "r") {
        event.preventDefault();
        onRefresh();
        return;
      }

      if (metaPressed && key === ",") {
        event.preventDefault();
        setCurrentView("settings");
        return;
      }

      if (event.key === "Escape" && !isEditableTarget(event.target)) {
        if (selectedPackage) {
          selectPackage(null);
        }
        if (searchQuery.trim()) {
          setSearchQuery("");
          setSearchResults(null);
          setSearchError(null);
        }
      }
    };

    window.addEventListener("keydown", handleShortcut);
    return () => window.removeEventListener("keydown", handleShortcut);
  }, [
    currentView,
    onRefresh,
    searchQuery,
    selectedPackage,
    selectPackage,
    setSearchQuery,
    setCurrentView
  ]);

  useEffect(() => {
    if (currentView !== "tasks") {
      return;
    }
    loadTasks();
    const timer = window.setInterval(() => {
      loadTasks();
    }, 1500);
    return () => window.clearInterval(timer);
  }, [currentView, loadTasks]);

  const onCancelTask = useCallback(
    async (taskId: string) => {
      if (cancelingTaskId) {
        return;
      }
      setCancelingTaskId(taskId);
      try {
        await cancelTask(taskId);
        await loadTasks();
      } catch (error) {
        console.error("Cancel task failed:", error);
      } finally {
        setCancelingTaskId(null);
      }
    },
    [cancelTask, cancelingTaskId, loadTasks]
  );

  const onCancelRunningTasks = useCallback(async () => {
    const runningTasks = tasks.filter((task) => task.status === "Running");
    if (runningTasks.length === 0) {
      return;
    }
    if (!window.confirm("确认终止所有运行中的任务吗？")) {
      return;
    }
    for (const task of runningTasks) {
      await onCancelTask(task.id);
    }
  }, [tasks, onCancelTask]);

  const onRemoveTask = useCallback(
    async (taskId: string) => {
      if (!window.confirm("确认删除这条任务记录吗？")) {
        return;
      }
      try {
        // removeTask 内部已经更新了前端状态，无需再次 loadTasks
        await removeTask(taskId);
      } catch (error) {
        console.error("Remove task failed:", error);
      }
    },
    [removeTask]
  );

  const onClearTasks = useCallback(async () => {
    if (!window.confirm("确认清空所有任务记录吗？")) {
      return;
    }
    try {
      await clearTasks();
      await loadTasks();
    } catch (error) {
      console.error("Clear tasks failed:", error);
    }
  }, [clearTasks, loadTasks]);

  return (
    <div className={`app-root ${sidebarCollapsed ? "app-collapsed" : ""}`}>
      <div className="titlebar-drag-region" data-tauri-drag-region />
      <aside className="sidebar">
        <div className="sidebar-logo">
          <div className="logo-circle">
            <img src="/logo.png" alt="Boxy logo" className="logo-image" />
          </div>
          <div className="logo-text">
            <span className="logo-title">boxy</span>
            <span className="logo-subtitle">package workspace</span>
          </div>
          <button
            className="sidebar-toggle"
            onClick={() => setSidebarCollapsed(!sidebarCollapsed)}
            aria-label="Toggle sidebar"
            type="button"
          >
            {sidebarCollapsed ? ">" : "<"}
          </button>
        </div>

        <nav className="sidebar-nav">
          {NAV_ITEMS.map((item) => (
            <button
              key={item.id}
              className={`nav-item ${currentView === item.id ? "nav-item-active" : ""}`}
              onClick={() => setCurrentView(item.id)}
              type="button"
            >
              <span className="nav-dot" />
              <span>{t(item.labelKey)}</span>
            </button>
          ))}
        </nav>

        {/* 主题与语言设置已移至 Settings 页面 */}
      </aside>

      <main className="main">
        <header className="main-header">
          <div className="main-header-inner">
            <div className="search-wrapper">
              <form className="search-box" onSubmit={onSearchSubmit}>
                <span className="search-prefix">{t("search.label")}</span>
                <input
                  className="search-input"
                  placeholder={t("search.placeholder")}
                  value={searchQuery}
                  ref={searchInputRef}
                  onChange={(event) => setSearchQuery(event.target.value)}
                />
                <span className="search-hint">Cmd + K</span>
              </form>
              {searchQuery.trim() && (
                <>
                  <div
                    className="search-panel-backdrop"
                    onClick={() => setSearchQuery("")}
                  />
                  <div className="search-panel">
                    <div className="search-panel-header">
                      <span>搜索结果</span>
                      {searching && <span className="search-panel-muted">搜索中...</span>}
                    </div>
                    {searchError && (
                      <div className="search-panel-empty">搜索失败：{searchError}</div>
                    )}
                    {!searchError && groupedSearchResults.length === 0 && !searching && (
                      <div className="search-panel-empty">未找到匹配包</div>
                    )}
                    {!searchError &&
                      groupedSearchResults.map((group) => (
                        <div key={group.manager} className="search-group">
                          <div className="search-group-title">{group.manager}</div>
                          <div className="search-group-list">
                            {group.items.map((pkg) => (
                              <button
                                key={`${pkg.manager}-${pkg.name}`}
                                type="button"
                                className="search-item"
                                onClick={() => {
                                  setCurrentView("manager");
                                  selectManager(pkg.manager);
                                  selectPackage(null);
                                  setSearchQuery("");
                                  setSearchResults(null);
                                  setSearchError(null);
                                }}
                              >
                                <span className="search-item-name">{pkg.name}</span>
                                <span className="search-item-meta">
                                  {pkg.version} · {pkg.manager}
                                </span>
                              </button>
                            ))}
                          </div>
                        </div>
                      ))}
                  </div>
                </>
              )}
            </div>
            <div className="top-controls">
              <button className="btn btn-secondary" type="button" onClick={onRefresh}>
                刷新
              </button>
              <button
                className="btn btn-ghost"
                type="button"
                onClick={() => setCurrentView("settings")}
              >
                设置
              </button>
            </div>
          </div>
        </header>

        <section className="main-content">
          {(packagesLoading || searching) && currentView === "manager" && (
            <LoadingIndicator label="正在加载..." />
          )}
          {currentView === "dashboard" && (
            <DashboardView
              managers={managers}
              tasks={tasks}
              onRefreshAll={onRefreshAllWithMessage}
              onOpenTasks={() => setCurrentView("tasks")}
              refreshMessage={refreshMessage}
            />
          )}
          {currentView === "manager" && (
            <ManagerView
              managers={managers}
              selectedManager={selectedManager}
              onSelectManager={selectManager}
              packages={filteredPackages}
              filter={filter}
              onFilterChange={setFilter}
              selectedPackage={selectedPackage}
              onSelectPackage={selectPackage}
              onBatchUpdate={onBatchUpdateRequest}
              batchUpdating={batchUpdating}
              batchUpdateMessage={batchUpdateMessage}
              searchQuery={searchQuery}
              searching={searching}
              searchError={searchError}
              onUpdatePackage={onUpdateSelectedPackage}
              onUninstallPackage={onUninstallSelectedPackage}
              packageActionKey={packageActionKey}
              packageActionMessage={packageActionMessage}
              packageScope={packageScope}
              packageDirectory={packageDirectory}
              onPackageScopeChange={setPackageScope}
              onPackageDirectoryChange={setPackageDirectory}
            />
          )}
          {currentView === "tasks" && (
            <TasksView
              tasks={tasks}
              onCancelTask={onCancelTask}
              onCancelRunningTasks={onCancelRunningTasks}
              onRemoveTask={onRemoveTask}
              onClearTasks={onClearTasks}
              cancelingTaskId={cancelingTaskId}
            />
          )}
          {currentView === "settings" && <SettingsView onOpenLogs={openLogs} />}
        </section>
      </main>

      <aside className="right-rail">
      <RightRailSection title="Active task">
        {currentTask ? (
          <div className="right-rail-body">
            <span className="pill pill-muted">{currentTask.operation}</span>
            <span className="right-rail-text">
              {currentTask.manager} · {currentTask.target}
            </span>
            <span className="right-rail-text">
              Status: {currentTask.status}
              {typeof currentTask.progress === "number"
                ? ` · ${Math.round(currentTask.progress)}%`
                : ""}
            </span>
          </div>
        ) : (
          <span className="right-rail-text">No active tasks</span>
        )}
      </RightRailSection>

        <RightRailSection title="Selected manager">
          {selectedManager ? (
            <div className="right-rail-body">
              <span className="pill pill-muted">{selectedManager}</span>
              <span className="right-rail-text">
                {packages.length} packages loaded
              </span>
            </div>
          ) : (
            <span className="right-rail-text">Select a manager</span>
          )}
        </RightRailSection>

        <RightRailSection title="Package info">
          {selectedPackage ? (
            <div className="right-rail-body">
              <span className="pill pill-muted">{selectedPackage.name}</span>
              {packageInfoLoading && (
                <span className="right-rail-text">加载中...</span>
              )}
              {packageInfoError && (
                <span className="right-rail-text">信息获取失败</span>
              )}
              {!packageInfoLoading && !packageInfoError && (
                <>
                  <span className="right-rail-text">
                    当前版本: {selectedPackage.version}
                  </span>
                  {(selectedPackageInfo?.latest_version ||
                    (selectedPackageInfo?.version &&
                      selectedPackageInfo.version !== selectedPackage.version)) && (
                    <span className="right-rail-text">
                      最新:{" "}
                      {selectedPackageInfo.latest_version ??
                        selectedPackageInfo?.version}
                    </span>
                  )}
                  {selectedPackageInfo?.description && (
                    <span className="right-rail-text">
                      简介: {selectedPackageInfo.description}
                    </span>
                  )}
                  {selectedPackageInfo?.license && (
                    <span className="right-rail-text">
                      License: {selectedPackageInfo.license}
                    </span>
                  )}
                  {typeof selectedPackageInfo?.size === "number" && (
                    <span className="right-rail-text">
                      大小: {formatSize(selectedPackageInfo.size)}
                    </span>
                  )}
                  {showHomepage && (
                    <button
                      type="button"
                      className="right-rail-link right-rail-link-button"
                      onClick={(event) => handleExternalLink(event, homepageLink)}
                    >
                      官网
                    </button>
                  )}
                  {showGit && (
                    <button
                      type="button"
                      className="right-rail-link right-rail-link-button"
                      onClick={(event) => handleExternalLink(event, gitLink)}
                    >
                      Git
                    </button>
                  )}
                  {selectedPackageInfo?.installed_path && (
                    <span className="right-rail-text">
                      路径: {selectedPackageInfo.installed_path}
                    </span>
                  )}
                </>
              )}
            </div>
          ) : (
            <span className="right-rail-text">请选择包查看详情</span>
          )}
        </RightRailSection>

        <RightRailSection title="Shortcuts">
          <div className="right-rail-body">
            <span className="right-rail-text">Cmd + K: search</span>
            <span className="right-rail-text">Cmd + R: refresh</span>
            <span className="right-rail-text">Cmd + ,: settings</span>
          </div>
        </RightRailSection>
      </aside>
      {updateDialog && (
        <div className="confirm-modal-backdrop" onClick={() => setUpdateDialog(null)}>
          <div className="confirm-modal" onClick={(event) => event.stopPropagation()}>
            <div className="confirm-modal-title">{updateDialog.title}</div>
            <div className="confirm-modal-text">{updateDialog.message}</div>
            <div className="confirm-modal-actions">
              {updateDialog.mode === "confirm" ? (
                <>
                  <button
                    type="button"
                    className="chip"
                    onClick={() => setUpdateDialog(null)}
                  >
                    取消
                  </button>
                  <button
                    type="button"
                    className="chip chip-danger"
                    onClick={async () => {
                      const payload = updateDialog;
                      setUpdateDialog(null);
                      if (payload.action === "single" && payload.pkg) {
                        await executeUpdatePackage(payload.pkg);
                      }
                      if (payload.action === "batch") {
                        await executeBatchUpdate();
                      }
                    }}
                  >
                    确认
                  </button>
                </>
              ) : (
                <button
                  type="button"
                  className="chip"
                  onClick={() => setUpdateDialog(null)}
                >
                  知道了
                </button>
              )}
            </div>
          </div>
        </div>
      )}
      {confirmUninstallPackage && (
        <div
          className="confirm-modal-backdrop"
          onClick={() => setConfirmUninstallPackage(null)}
        >
          <div
            className="confirm-modal"
            onClick={(event) => event.stopPropagation()}
          >
            <div className="confirm-modal-title">确认卸载</div>
            <div className="confirm-modal-text">
              确认卸载 {confirmUninstallPackage.name} 吗？
            </div>
            <div className="confirm-modal-actions">
              <button
                type="button"
                className="chip"
                onClick={() => setConfirmUninstallPackage(null)}
              >
                取消
              </button>
              <button
                type="button"
                className="chip chip-danger"
                onClick={async () => {
                  const pkg = confirmUninstallPackage;
                  setConfirmUninstallPackage(null);
                  if (pkg) {
                    await executeUninstallPackage(pkg);
                  }
                }}
              >
                确认卸载
              </button>
            </div>
          </div>
        </div>
      )}
      {logsOpen && (
        <div className="logs-modal-backdrop" onClick={() => setLogsOpen(false)}>
          <div className="logs-modal" onClick={(event) => event.stopPropagation()}>
            <div className="logs-modal-header">
              <span>应用日志</span>
              <button
                type="button"
                className="chip"
                onClick={() => setLogsOpen(false)}
              >
                关闭
              </button>
            </div>
            <div className="logs-toolbar">
              <input
                className="logs-search"
                type="text"
                value={logFilter}
                onChange={(event) => setLogFilter(event.target.value)}
                placeholder="搜索日志"
              />
              <div className="logs-levels">
                {["all", "INFO", "WARN", "ERROR"].map((level) => (
                  <button
                    key={level}
                    type="button"
                    className={`chip ${logLevelFilter === level ? "chip-active" : ""}`}
                    onClick={() => setLogLevelFilter(level)}
                  >
                    {level === "all" ? "全部" : level}
                  </button>
                ))}
              </div>
            </div>
            {logPath && <div className="logs-path">路径: {logPath}</div>}
            {logsLoading && <div className="logs-empty">加载中...</div>}
            {logsError && <div className="logs-empty">加载失败: {logsError}</div>}
            {!logsLoading && !logsError && logsLines.length === 0 && (
              <div className="logs-empty">暂无日志</div>
            )}
            {!logsLoading && !logsError && logsLines.length > 0 && (
              <div className="logs-content">
                {displayLogs.map((entry, index) => (
                  <div
                    key={`${entry.raw}-${index}`}
                    className={`log-line log-${entry.level.toLowerCase()}`}
                  >
                    {entry.text}
                  </div>
                ))}
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
};

const DashboardView: React.FC<{
  managers: ManagerStatus[];
  tasks: Job[];
  onRefreshAll: () => void;
  onOpenTasks: () => void;
  refreshMessage: string | null;
}> = ({ managers, tasks, onRefreshAll, onOpenTasks, refreshMessage }) => {
  const totalPackages = managers.reduce((sum, manager) => sum + manager.package_count, 0);
  const outdatedPackages = managers.reduce((sum, manager) => sum + manager.outdated_count, 0);
  const runningTasks = tasks.filter((task) => task.status === "Running").length;

  return (
    <>
      <section className="hero">
        <div className="hero-text">
          <h1>Unified package workspace</h1>
          <p>Track packages across managers, keep everything updated, ship faster.</p>
          <div className="hero-actions">
            <button className="btn btn-primary" type="button" onClick={onRefreshAll}>
              刷新全部
            </button>
            <button className="btn btn-ghost" type="button" onClick={onOpenTasks}>
              查看任务
            </button>
          </div>
          {refreshMessage && (
            <span className="control-hint">{refreshMessage}</span>
          )}
        </div>
        <div className="hero-panel">
          <div className="hero-panel-header">
            <span className="dot dot-green" />
            <span className="dot dot-amber" />
            <span className="dot dot-red" />
          </div>
          <div className="hero-panel-body">
            <div className="hero-line hero-line-main">$ boxy scan</div>
            <div className="hero-line hero-line-faded">brew 124 packages</div>
            <div className="hero-line hero-line-faded">npm 256 packages</div>
            <div className="hero-line hero-line-faded">pnpm 89 packages</div>
          </div>
        </div>
      </section>

      <section className="grid-row">
        <div className="card">
          <div className="card-header">
            <h2>Managers</h2>
            <span className="card-subtitle">{managers.length} total</span>
          </div>
          <div className="manager-grid">
            {managers.map((manager) => (
              <ManagerPill key={manager.name} manager={manager} />
            ))}
          </div>
        </div>
        <div className="card">
          <div className="card-header">
            <h2>Overview</h2>
            <span className="card-subtitle">Package stats</span>
          </div>
          <div className="right-rail-body">
            <span className="pill pill-muted">Total packages: {totalPackages}</span>
            <span className="pill pill-muted">Outdated packages: {outdatedPackages}</span>
            <span className="pill pill-muted">Running tasks: {runningTasks}</span>
          </div>
        </div>
      </section>
    </>
  );
};

const ManagerView: React.FC<{
  managers: ManagerStatus[];
  selectedManager: string | null;
  onSelectManager: (manager: string | null) => void;
  packages: Package[];
  filter: "all" | "outdated";
  onFilterChange: (filter: "all" | "outdated") => void;
  selectedPackage: Package | null;
  onSelectPackage: (pkg: Package | null) => void;
  onBatchUpdate: (hasOutdated: boolean) => void;
  batchUpdating: boolean;
  batchUpdateMessage: string | null;
  searchQuery: string;
  searching: boolean;
  searchError: string | null;
  onUpdatePackage: (pkg: Package) => void;
  onUninstallPackage: (pkg: Package) => void;
  packageActionKey: string | null;
  packageActionMessage: string | null;
  packageScope: "global" | "local";
  packageDirectory: string;
  onPackageScopeChange: (scope: "global" | "local") => void;
  onPackageDirectoryChange: (directory: string) => void;
}> = ({
  managers,
  selectedManager,
  onSelectManager,
  packages,
  filter,
  onFilterChange,
  selectedPackage,
  onSelectPackage,
  onBatchUpdate,
  batchUpdating,
  batchUpdateMessage,
  searchQuery,
  searching,
  searchError,
  onUpdatePackage,
  onUninstallPackage,
  packageActionKey,
  packageActionMessage,
  packageScope,
  packageDirectory,
  onPackageScopeChange,
  onPackageDirectoryChange
}) => {
  const hasOutdated = packages.some((pkg) => pkg.outdated);
  const sortedPackages = [...packages].sort((left, right) => {
    if (left.outdated === right.outdated) {
      return left.name.localeCompare(right.name);
    }
    return left.outdated ? -1 : 1;
  });
  const [directoryInput, setDirectoryInput] = useState(packageDirectory);

  useEffect(() => {
    setDirectoryInput(packageDirectory);
  }, [packageDirectory]);

  const applyDirectory = () => {
    onPackageDirectoryChange(directoryInput.trim());
  };

  return (
    <>
      <div className="card">
        <div className="card-header">
          <h2>Manager packages</h2>
          <span className="card-subtitle">{selectedManager ?? "None"}</span>
        </div>
        <div className="control-buttons">
          {managers.map((manager) => (
            <button
              key={manager.name}
              className={`chip ${selectedManager === manager.name ? "chip-active" : ""}`}
              onClick={() => onSelectManager(manager.name)}
              type="button"
            >
              {manager.name}
              {manager.outdated_count > 0 && (
                <span className="chip-badge">{manager.outdated_count}</span>
              )}
            </button>
          ))}
        </div>
        <div className="manager-scope">
          <span className="control-label">检测范围</span>
          <div className="control-buttons">
            <button
              className={`chip ${packageScope === "global" ? "chip-active" : ""}`}
              type="button"
              onClick={() => {
                onPackageScopeChange("global");
                onPackageDirectoryChange("");
              }}
            >
              全局
            </button>
            <button
              className={`chip ${packageScope === "local" ? "chip-active" : ""}`}
              type="button"
              onClick={() => onPackageScopeChange("local")}
            >
              目录
            </button>
          </div>
          {packageScope === "local" && (
            <div className="manager-scope-input">
              <input
                className="scope-input"
                type="text"
                value={directoryInput}
                onChange={(event) => setDirectoryInput(event.target.value)}
                placeholder="输入目录路径，例如 /Users/you/project"
              />
              <button
                className="chip"
                type="button"
                onClick={applyDirectory}
                disabled={!directoryInput.trim()}
              >
                应用
              </button>
              {!packageDirectory.trim() && (
                <span className="control-hint">未设置目录，列表不会加载。</span>
              )}
            </div>
          )}
        </div>
      </div>

      <div className="card">
        <div className="card-header">
          <h2>Packages</h2>
          <div className="control-buttons">
            <button
              className={`chip ${filter === "all" ? "chip-active" : ""}`}
              onClick={() => onFilterChange("all")}
              type="button"
            >
              All
            </button>
            <button
              className={`chip ${filter === "outdated" ? "chip-active" : ""}`}
              onClick={() => onFilterChange("outdated")}
              type="button"
            >
              Outdated
            </button>
            <button
              className="chip"
              type="button"
              onClick={() => onBatchUpdate(hasOutdated)}
              disabled={!selectedManager || batchUpdating}
            >
              {batchUpdating ? "更新中..." : "一键全部更新"}
            </button>
            {batchUpdateMessage && (
              <span className="control-hint">{batchUpdateMessage}</span>
            )}
          </div>
        </div>

        <div className="package-list">
          {sortedPackages.map((pkg) => (
            <div
              key={`${pkg.manager}-${pkg.name}`}
              className={`package-item ${pkg.outdated ? "package-item-outdated" : ""}`}
              onClick={() => onSelectPackage(pkg)}
              onKeyDown={(event) => {
                if (event.key === "Enter" || event.key === " ") {
                  event.preventDefault();
                  onSelectPackage(pkg);
                }
              }}
              role="button"
              tabIndex={0}
              aria-label={`Select package ${pkg.name}`}
            >
              <div className="package-meta">
                <span className="package-name">{pkg.name}</span>
                <span className="package-version">
                  {pkg.version} · {pkg.manager}
                  {typeof pkg.size === "number" ? ` · ${formatSize(pkg.size)}` : ""}
                </span>
              </div>
              <div className="package-actions">
                {selectedPackage?.name === pkg.name &&
                  selectedPackage?.manager === pkg.manager && (
                    <div className="package-action-buttons">
                      <button
                        className="chip"
                        type="button"
                        onClick={(event) => {
                          event.stopPropagation();
                          onUpdatePackage(pkg);
                        }}
                        disabled={
                          packageActionKey !== null &&
                          packageActionKey !== `${pkg.manager}:${pkg.name}:update`
                        }
                      >
                        {packageActionKey === `${pkg.manager}:${pkg.name}:update`
                          ? "更新中..."
                          : "更新"}
                      </button>
                      <button
                        className="chip chip-danger"
                        type="button"
                        onClick={(event) => {
                          event.stopPropagation();
                          onUninstallPackage(pkg);
                        }}
                        disabled={
                          packageActionKey !== null &&
                          packageActionKey !== `${pkg.manager}:${pkg.name}:uninstall`
                        }
                      >
                        {packageActionKey === `${pkg.manager}:${pkg.name}:uninstall`
                          ? "卸载中..."
                          : "卸载"}
                      </button>
                    </div>
                  )}
                {pkg.outdated && <span className="badge badge-warn">Outdated</span>}
                {selectedPackage?.name === pkg.name &&
                  selectedPackage?.manager === pkg.manager && (
                  <span className="badge">Selected</span>
                )}
              </div>
            </div>
          ))}
          {packageActionMessage && (
            <div className="control-hint">{packageActionMessage}</div>
          )}
          {searchError && (
            <div className="tasks-empty">
              <div className="tasks-empty-title">搜索失败</div>
              <div className="tasks-empty-text">{searchError}</div>
            </div>
          )}
          {!searchError && packages.length === 0 && (
            <div className="tasks-empty">
              <div className="tasks-empty-title">
                {searchQuery.trim() ? "未找到匹配包" : "No packages"}
              </div>
              <div className="tasks-empty-text">
                {searchQuery.trim()
                  ? "请调整搜索关键词或选择其他管理器。"
                  : "Select a manager to load packages."}
              </div>
            </div>
          )}
          {searching && (
            <div className="control-hint">搜索中...</div>
          )}
        </div>
      </div>
    </>
  );
};

const TasksView: React.FC<{
  tasks: Job[];
  onCancelTask: (taskId: string) => void;
  onCancelRunningTasks: () => void;
  onRemoveTask: (taskId: string) => void;
  onClearTasks: () => void;
  cancelingTaskId: string | null;
}> = ({ tasks, onCancelTask, onCancelRunningTasks, onRemoveTask, onClearTasks, cancelingTaskId }) => {
  const runningTasks = tasks.filter((task) => task.status === "Running");
  return (
    <div className="card">
      <div className="card-header">
        <h2>Tasks</h2>
        <div className="task-header-actions">
          {runningTasks.length > 0 && (
            <button
              type="button"
              className="chip chip-danger"
              onClick={onCancelRunningTasks}
              disabled={cancelingTaskId !== null}
            >
              终止任务
            </button>
          )}
          {tasks.length > 0 && (
            <button type="button" className="chip" onClick={onClearTasks}>
              删除任务信息
            </button>
          )}
          <span className="card-subtitle">{tasks.length} tasks</span>
        </div>
      </div>
      {tasks.length === 0 ? (
        <div className="tasks-empty">
          <div className="tasks-empty-title">No tasks</div>
          <div className="tasks-empty-text">Your running tasks appear here.</div>
        </div>
      ) : (
        <div className="task-list">
          {tasks.map((task) => (
            <div key={task.id} className="task-item">
              <div className="task-meta">
                <span className="task-title">
                  {task.operation} {task.manager} {task.target}
                </span>
                <span className="task-status">
                  {task.status}
                  {typeof task.progress === "number"
                    ? ` · ${Math.round(task.progress)}%`
                    : ""}
                </span>
              </div>
              {task.status === "Running" && (
                <button
                  type="button"
                  className="chip chip-danger"
                  onClick={() => onCancelTask(task.id)}
                  disabled={cancelingTaskId !== null && cancelingTaskId !== task.id}
                >
                  {cancelingTaskId === task.id ? "取消中..." : "取消"}
                </button>
              )}
              {task.status !== "Running" && (
                <button
                  type="button"
                  className="chip"
                  onClick={() => onRemoveTask(task.id)}
                >
                  删除
                </button>
              )}
            </div>
          ))}
        </div>
      )}
    </div>
  );
};

type UpdateStatus =
  | "idle"
  | "checking"
  | "available"
  | "downloading"
  | "installing"
  | "done"
  | "error";

type UpdateHandle = Update | null;

const SettingsView: React.FC<{ onOpenLogs: () => void }> = ({ onOpenLogs }) => {
  const { theme, setTheme } = useTheme();
  const { locale, setLocale, t } = useI18n();
  const [appVersion, setAppVersion] = useState<string>("");
  const gitUrl = "https://github.com/ljiulong/boxyy";
  const releasesUrl = "https://github.com/ljiulong/boxyy/releases/latest";
  const updateRef = useRef<UpdateHandle>(null);
  const [updateStatus, setUpdateStatus] = useState<UpdateStatus>("idle");
  const [updateProgress, setUpdateProgress] = useState<number | null>(null);
  const [updateVersion, setUpdateVersion] = useState<string | null>(null);
  const [updateMessage, setUpdateMessage] = useState<string | null>(null);
  const totalBytesRef = useRef<number | null>(null);
  const downloadedBytesRef = useRef(0);

  const handleUpdaterEvent = useCallback((event: DownloadEvent) => {
    if (event.event === "Started") {
      totalBytesRef.current = event.data.contentLength ?? null;
      downloadedBytesRef.current = 0;
      setUpdateStatus("downloading");
      setUpdateProgress(0);
      return;
    }
    if (event.event === "Progress") {
      downloadedBytesRef.current += event.data.chunkLength;
      const total = totalBytesRef.current;
      if (typeof total === "number" && total > 0) {
        const next = Math.min(
          100,
          Math.round((downloadedBytesRef.current / total) * 100)
        );
        setUpdateProgress(next);
      }
      return;
    }
    if (event.event === "Finished") {
      setUpdateProgress(100);
    }
  }, []);

  const checkForUpdate = useCallback(
    async (silent = false) => {
      if (!isTauri()) {
        return;
      }
      if (!silent) {
        setUpdateStatus("checking");
        setUpdateProgress(null);
        setUpdateMessage(null);
      }
      try {
        const update = (await check()) as UpdateHandle;
        if (update) {
          updateRef.current = update;
          setUpdateVersion(update.version ?? null);
          setUpdateStatus("available");
          return;
        }
        updateRef.current = null;
        setUpdateVersion(null);
        setUpdateStatus("idle");
      } catch (error) {
        console.error("Check update failed:", error);
        setUpdateStatus("error");
        setUpdateMessage(t("settings.about.check_failed"));
      }
    },
    [t]
  );

  useEffect(() => {
    if (!isTauri()) {
      setAppVersion("dev");
      return;
    }
    getVersion()
      .then((version) => {
        setAppVersion(version);
      })
      .catch((error) => {
        console.error("Get app version failed:", error);
        setAppVersion("unknown");
      });

    checkForUpdate(true);
  }, []);

  const onOpenGit = async (event: React.MouseEvent<HTMLAnchorElement>) => {
    event.preventDefault();
    try {
      await openExternalUrl(gitUrl);
    } catch (error) {
      console.error("Open git url failed:", error);
      window.open(gitUrl, "_blank", "noreferrer");
    }
  };

  const onCheckUpdate = useCallback(async () => {
    if (!isTauri()) {
      window.open(releasesUrl, "_blank", "noreferrer");
      return;
    }
    if (updateStatus === "checking" || updateStatus === "downloading") {
      return;
    }
    if (updateStatus === "installing") {
      return;
    }
    if (updateStatus === "available") {
      setUpdateStatus("downloading");
      setUpdateProgress(0);
      setUpdateMessage(null);
      try {
        const update = updateRef.current ?? ((await check()) as UpdateHandle);
        if (!update || typeof update.downloadAndInstall !== "function") {
          setUpdateStatus("idle");
          return;
        }
        setUpdateVersion(update.version ?? updateVersion);
        await update.downloadAndInstall(handleUpdaterEvent);
        setUpdateStatus("done");
        setUpdateMessage(t("settings.about.update_ready"));
        try {
          await relaunch();
        } catch (error) {
          console.warn("Relaunch failed:", error);
        }
      } catch (error) {
        console.error("Install update failed:", error);
        setUpdateStatus("error");
        setUpdateMessage(t("settings.about.update_failed"));
      }
      return;
    }
    await checkForUpdate();
  }, [checkForUpdate, handleUpdaterEvent, releasesUrl, t, updateStatus]);

  const updateLabel = useMemo(() => {
    if (updateStatus === "available" && updateVersion) {
      return t("settings.about.update_action", { version: updateVersion });
    }
    if (updateStatus === "checking") {
      return t("settings.about.update_checking");
    }
    if (updateStatus === "downloading") {
      return t("settings.about.update_downloading");
    }
    if (updateStatus === "installing") {
      return t("settings.about.update_installing");
    }
    return t("settings.about.check_update");
  }, [t, updateStatus, updateVersion]);

  const updateStatusText = useMemo(() => {
    if (updateStatus === "available" && updateVersion) {
      return t("settings.about.update_available", { version: updateVersion });
    }
    if (updateStatus === "checking") {
      return t("settings.about.update_checking");
    }
    if (updateStatus === "downloading") {
      return t("settings.about.update_downloading");
    }
    if (updateStatus === "installing") {
      return t("settings.about.update_installing");
    }
    if (updateStatus === "done") {
      return t("settings.about.update_ready");
    }
    if (updateStatus === "error") {
      return updateMessage ?? t("settings.about.update_failed");
    }
    if (updateStatus === "idle") {
      return t("settings.about.update_latest");
    }
    return null;
  }, [t, updateMessage, updateStatus, updateVersion]);

  return (
    <div className="settings-stack">
      <div className="card">
        <div className="card-header">
          <h2>{t("settings.title")}</h2>
          <span className="card-subtitle">{t("settings.subtitle")}</span>
        </div>
        <div className="right-rail-body">
          <div className="control-group">
            <span className="control-label">{t("settings.language")}</span>
            <div className="control-buttons">
              <button
                type="button"
                className={`chip ${locale === "zh" ? "chip-active" : ""}`}
                onClick={() => setLocale("zh")}
              >
                {t("settings.language.zh")}
              </button>
              <button
                type="button"
                className={`chip ${locale === "en" ? "chip-active" : ""}`}
                onClick={() => setLocale("en")}
              >
                {t("settings.language.en")}
              </button>
            </div>
          </div>

          <div className="control-group">
            <span className="control-label">{t("settings.theme")}</span>
            <div className="control-buttons">
              {(["light", "dark", "system"] as const).map((mode) => (
                <button
                  key={mode}
                  type="button"
                  className={`chip ${theme === mode ? "chip-active" : ""}`}
                  onClick={() => setTheme(mode)}
                >
                  {t(`settings.theme.${mode}`)}
                </button>
              ))}
            </div>
          </div>

          <div className="control-group">
            <span className="control-label">{t("settings.logs")}</span>
            <div className="control-buttons">
              <button type="button" className="chip" onClick={onOpenLogs}>
                {t("settings.logs.open")}
              </button>
            </div>
          </div>
        </div>
      </div>

      <div className="card">
        <div className="card-header">
          <h2>{t("settings.about.title")}</h2>
          <span className="card-subtitle">{t("settings.about.subtitle")}</span>
        </div>
        <div className="right-rail-body">
          <div className="about-row">
            <span className="about-label">{t("settings.about.app_name")}</span>
            <span className="about-value">Boxy</span>
          </div>
          <div className="about-row">
            <span className="about-label">{t("settings.about.version")}</span>
            <span className="about-value">{appVersion || "..."}</span>
          </div>
          <div className="about-row">
            <span className="about-label">{t("settings.about.git")}</span>
            <a className="about-icon-link" href={gitUrl} onClick={onOpenGit}>
              <svg
                className="about-icon"
                viewBox="0 0 24 24"
                aria-hidden="true"
                focusable="false"
              >
                <path
                  fill="currentColor"
                  d="M12 1.5c-5.8 0-10.5 4.7-10.5 10.5 0 4.7 3 8.6 7.2 10 .5.1.7-.2.7-.5v-1.8c-2.9.6-3.5-1.4-3.5-1.4-.5-1.2-1.1-1.5-1.1-1.5-.9-.6.1-.6.1-.6 1 .1 1.5 1 1.5 1 .9 1.5 2.4 1 3 .8.1-.7.4-1 .7-1.3-2.3-.3-4.7-1.1-4.7-5 0-1.1.4-2 1-2.7-.1-.2-.4-1.3.1-2.7 0 0 .9-.3 2.8 1 .8-.2 1.7-.3 2.6-.3.9 0 1.8.1 2.6.3 1.9-1.3 2.8-1 2.8-1 .5 1.4.2 2.5.1 2.7.6.7 1 1.6 1 2.7 0 3.9-2.4 4.7-4.7 5 .4.3.8 1 .8 2.1v3.1c0 .3.2.6.7.5 4.2-1.4 7.2-5.3 7.2-10C22.5 6.2 17.8 1.5 12 1.5z"
                />
              </svg>
            </a>
          </div>
          <div className="control-buttons">
            <button
              type="button"
              className="chip"
              onClick={onCheckUpdate}
              disabled={
                updateStatus === "checking" ||
                updateStatus === "downloading" ||
                updateStatus === "installing"
              }
            >
              {updateLabel}
            </button>
          </div>
          {updateStatusText && (
            <div className="update-status">
              <span>{updateStatusText}</span>
              {typeof updateProgress === "number" && (
                <span>{` ${updateProgress}%`}</span>
              )}
            </div>
          )}
          {typeof updateProgress === "number" && (
            <div className="update-progress">
              <div
                className="update-progress-bar"
                style={{ width: `${updateProgress}%` }}
              />
            </div>
          )}
        </div>
      </div>
    </div>
  );
};

const RightRailSection: React.FC<{ title: string; children: React.ReactNode }> = ({
  title,
  children
}) => {
  return (
    <section className="right-rail-section">
      <div className="right-rail-title">{title}</div>
      {children}
    </section>
  );
};

const LoadingIndicator: React.FC<{ label: string }> = ({ label }) => {
  return <div className="loading-indicator">{label}</div>;
};

const ManagerPill: React.FC<{ manager: ManagerStatus }> = ({ manager }) => {
  const statusClass = manager.available
    ? manager.outdated_count > 0
      ? "warn"
      : "ok"
    : "error";

  const dotClass = manager.available
    ? manager.outdated_count > 0
      ? "manager-dot-warning"
      : "manager-dot-muted"
    : "manager-dot-error";

  const pillClass = manager.outdated_count > 0 ? "manager-pill manager-pill-alert" : "manager-pill";
  return (
    <div className={pillClass}>
      <span className={dotClass} />
      <span className="manager-name">{manager.name}</span>
      <span className={`manager-status ${statusClass}`}>
        {manager.available ? "online" : "offline"}
      </span>
      {manager.outdated_count > 0 && (
        <span className="manager-alert">
          可更新 {manager.outdated_count}
        </span>
      )}
    </div>
  );
};
