import { invoke } from "@tauri-apps/api/core";
import type { Job, ManagerStatus, Package } from "../types";
import {
  mockManagers,
  mockPackages,
  clearMockTasks,
  getMockTasks,
  removeMockTask
} from "./mock";

interface TauriWindow extends Window {
  __TAURI_INTERNALS__?: unknown;
}

export const isTauri = (): boolean => {
  if (typeof window === "undefined") {
    return false;
  }
  const tauriWindow = window as TauriWindow;
  return !!tauriWindow.__TAURI_INTERNALS__;
};

export async function scanManagers(): Promise<ManagerStatus[]> {
  if (!isTauri()) {
    return mockManagers;
  }
  return invoke<ManagerStatus[]>("scan_managers");
}

export async function getManagerPackages(
  manager: string,
  scope?: string,
  directory?: string,
  force?: boolean
): Promise<Package[]> {
  if (!isTauri()) {
    return mockPackages.filter((pkg) => pkg.manager === manager);
  }
  return invoke<Package[]>("get_manager_packages", { manager, scope, directory, force });
}

export async function refreshManager(
  manager: string,
  scope?: string,
  directory?: string
): Promise<void> {
  if (!isTauri()) {
    return;
  }
  await invoke("refresh_manager", { manager, scope, directory });
}

export async function searchPackages(query: string, manager?: string): Promise<Package[]> {
  if (!isTauri()) {
    return mockPackages.filter((pkg) =>
      pkg.name.toLowerCase().includes(query.toLowerCase())
    );
  }
  return invoke<Package[]>("search_packages", { query, manager });
}

export async function getPackageInfo(
  manager: string,
  packageName: string,
  scope?: string,
  directory?: string
): Promise<Package> {
  if (!isTauri()) {
    const packages = mockPackages ?? [];
    const match = packages.find(
      (pkg) => pkg.manager === manager && pkg.name === packageName
    );
    if (!match) {
      throw new Error(`Package not found: ${manager}/${packageName}`);
    }
    return match;
  }
  return invoke<Package>("get_package_info", {
    manager,
    package: packageName,
    scope,
    directory
  });
}

export async function installPackage(
  manager: string,
  packageName: string,
  version?: string,
  scope?: string,
  directory?: string
): Promise<string> {
  if (!isTauri()) {
    return "mock-task-install";
  }
  return invoke<string>("install_package", {
    manager,
    package: packageName,
    version,
    scope,
    directory
  });
}

export async function updatePackage(
  manager: string,
  packageName: string,
  scope?: string,
  directory?: string
): Promise<string> {
  if (!isTauri()) {
    return "mock-task-update";
  }
  return invoke<string>("update_package", { manager, package: packageName, scope, directory });
}

export async function updateOutdatedPackages(
  manager: string,
  scope?: string,
  directory?: string
): Promise<string> {
  if (!isTauri()) {
    return "mock-task-update-outdated";
  }
  return invoke<string>("update_outdated_packages", { manager, scope, directory });
}

export async function uninstallPackage(
  manager: string,
  packageName: string,
  force = false,
  scope?: string,
  directory?: string
): Promise<string> {
  if (!isTauri()) {
    return "mock-task-uninstall";
  }
  return invoke<string>("uninstall_package", {
    manager,
    package: packageName,
    force,
    scope,
    directory
  });
}

export async function getTasks(): Promise<Job[]> {
  if (!isTauri()) {
    return getMockTasks();
  }
  return invoke<Job[]>("get_tasks");
}

export async function getTaskLogs(taskId: string): Promise<string[]> {
  if (!isTauri()) {
    return ["Mock log line 1", "Mock log line 2"];
  }
  return invoke<string[]>("get_task_logs", { taskId });
}

export async function cancelTask(taskId: string): Promise<void> {
  if (!isTauri()) {
    return;
  }
  await invoke("cancel_task", { taskId });
}

export async function openExternalUrl(url: string): Promise<void> {
  if (!isTauri()) {
    const opened = window.open(url, "_blank", "noopener,noreferrer");
    if (!opened) {
      window.location.assign(url);
    }
    return;
  }
  await invoke("open_external_url", { url });
}

export async function deleteTask(taskId: string): Promise<void> {
  if (!isTauri()) {
    removeMockTask(taskId);
    return;
  }
  await invoke("delete_task", { taskId });
}

export async function clearTasks(): Promise<void> {
  if (!isTauri()) {
    clearMockTasks();
    return;
  }
  await invoke("clear_tasks");
}

export async function getAppLogs(): Promise<string[]> {
  if (!isTauri()) {
    return ["{\"level\":\"info\",\"message\":\"Mock log\"}"];
  }
  return invoke<string[]>("get_app_logs");
}

export async function appendFrontendLog(level: string, message: string): Promise<void> {
  if (!isTauri()) {
    return;
  }
  await invoke("append_frontend_log", { level, message });
}

export async function getAppLogPath(): Promise<string> {
  if (!isTauri()) {
    return "/tmp/boxy.log";
  }
  return invoke<string>("get_app_log_path");
}
