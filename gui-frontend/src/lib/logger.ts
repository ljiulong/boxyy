import { appendFrontendLog, isTauri } from "./api";

let isInitialized = false;
let isReporting = false;

const serializeArgs = (args: unknown[]): string => {
  return args
    .map((arg) => {
      if (typeof arg === "string") {
        return arg;
      }
      try {
        return JSON.stringify(arg);
      } catch {
        return String(arg);
      }
    })
    .join(" ");
};

export const initFrontendLogger = () => {
  if (isInitialized || !isTauri()) {
    return;
  }
  isInitialized = true;

  const originalLog = console.log;
  const originalWarn = console.warn;
  const originalError = console.error;

  // 包装 console 输出，写入后端日志
  console.log = (...args: unknown[]) => {
    originalLog(...args);
    reportLog("info", serializeArgs(args));
  };
  console.warn = (...args: unknown[]) => {
    originalWarn(...args);
    reportLog("warn", serializeArgs(args));
  };
  console.error = (...args: unknown[]) => {
    originalError(...args);
    reportLog("error", serializeArgs(args));
  };

  window.addEventListener("error", (event) => {
    reportLog("error", event.message || "未知错误");
  });

  window.addEventListener("unhandledrejection", (event) => {
    reportLog("error", String(event.reason ?? "未知 Promise 错误"));
  });
};

const reportLog = (level: string, message: string) => {
  if (isReporting) {
    return;
  }
  isReporting = true;
  appendFrontendLog(level, message)
    .catch(() => {
      // 忽略日志写入失败
    })
    .finally(() => {
      isReporting = false;
    });
};
