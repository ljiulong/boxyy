import { useCallback } from "react";
import { useAppStore } from "../store/appStore";

type Locale = "zh" | "en";

const messages: Record<Locale, Record<string, string>> = {
  zh: {
    "nav.dashboard": "概览",
    "nav.manager": "管理器",
    "nav.tasks": "任务",
    "nav.settings": "设置",

    "search.label": "搜索",
    "search.placeholder": "搜索包、查看更新或执行安装操作…",

    "settings.title": "设置",
    "settings.subtitle": "个性化 Boxy",
    "settings.language": "语言",
    "settings.language.zh": "中文",
    "settings.language.en": "English",
    "settings.theme": "主题",
    "settings.theme.light": "浅色",
    "settings.theme.dark": "深色",
    "settings.theme.system": "跟随系统",
    "settings.logs": "日志",
    "settings.logs.open": "打开任务日志",
    "settings.about.title": "关于",
    "settings.about.subtitle": "应用信息",
    "settings.about.app_name": "应用",
    "settings.about.version": "版本",
    "settings.about.git": "Git 地址",
    "settings.about.check_update": "检测更新",
    "settings.about.update_checking": "检查更新中…",
    "settings.about.update_latest": "已是最新版本",
    "settings.about.update_available": "发现新版本 {version}",
    "settings.about.update_downloading": "正在下载更新…",
    "settings.about.update_installing": "正在安装更新…",
    "settings.about.update_ready": "更新完成，正在重启…",
    "settings.about.update_failed": "更新失败，请重试",
    "settings.about.check_failed": "检测失败，建议手动前往 GitHub 页面查看",
    "settings.about.update_action": "更新到 {version}"
  },
  en: {
    "nav.dashboard": "Overview",
    "nav.manager": "Managers",
    "nav.tasks": "Tasks",
    "nav.settings": "Settings",

    "search.label": "Search",
    "search.placeholder": "Search packages, check updates, or run installs…",

    "settings.title": "Settings",
    "settings.subtitle": "Personalize Boxy",
    "settings.language": "Language",
    "settings.language.zh": "中文",
    "settings.language.en": "English",
    "settings.theme": "Theme",
    "settings.theme.light": "Light",
    "settings.theme.dark": "Dark",
    "settings.theme.system": "System",
    "settings.logs": "Logs",
    "settings.logs.open": "Open task logs",
    "settings.about.title": "About",
    "settings.about.subtitle": "App details",
    "settings.about.app_name": "App",
    "settings.about.version": "Version",
    "settings.about.git": "Git",
    "settings.about.check_update": "Check update",
    "settings.about.update_checking": "Checking for updates…",
    "settings.about.update_latest": "You're up to date",
    "settings.about.update_available": "Update available {version}",
    "settings.about.update_downloading": "Downloading update…",
    "settings.about.update_installing": "Installing update…",
    "settings.about.update_ready": "Update complete, restarting…",
    "settings.about.update_failed": "Update failed, please retry",
    "settings.about.check_failed": "Check failed, please visit GitHub releases manually",
    "settings.about.update_action": "Update to {version}"
  },
};

export const useI18n = () => {
  const locale = useAppStore((state) => state.locale);
  const setLocale = useAppStore((state) => state.setLocale);

  const t = useCallback(
    (key: string, params?: Record<string, string | number>) => {
      const template = messages[locale][key] ?? key;
      if (!params) {
        return template;
      }
      return Object.entries(params).reduce((acc, [k, v]) => {
        return acc.replace(`{${k}}`, String(v));
      }, template);
    },
    [locale]
  );

  return { locale, setLocale, t };
};


