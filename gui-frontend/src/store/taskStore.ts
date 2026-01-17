import { create } from "zustand";
import type { Job } from "../types";
import {
  cancelTask as cancelTaskApi,
  clearTasks as clearTasksApi,
  deleteTask as deleteTaskApi,
  getTasks,
  getTaskLogs
} from "../lib/api";

type TaskState = {
  tasks: Job[];
  currentTask: Job | null;
  logs: Record<string, string[]>;
  hiddenTaskIds: string[];
  clearedAt: string | null;
  loadTasks: () => Promise<void>;
  loadTaskLogs: (taskId: string) => Promise<void>;
  cancelTask: (taskId: string) => Promise<void>;
  removeTask: (taskId: string) => Promise<void>;
  clearTasks: () => Promise<void>;
  addTask: (task: Job) => void;
  updateTask: (taskId: string, updates: Partial<Job>) => void;
};

const pickRunningTask = (tasks: Job[]): Job | null =>
  tasks.find((task) => task.status === "Running") ?? null;

const isAfterClearedAt = (task: Job, clearedAt: string | null): boolean => {
  if (!clearedAt) {
    return true;
  }
  if (!task.started_at) {
    return false;
  }
  const taskTime = Date.parse(task.started_at);
  const clearedTime = Date.parse(clearedAt);
  if (Number.isNaN(taskTime) || Number.isNaN(clearedTime)) {
    return false;
  }
  return taskTime > clearedTime;
};

export const useTaskStore = create<TaskState>((set, get) => ({
  tasks: [],
  currentTask: null,
  logs: {},
  hiddenTaskIds: [],
  clearedAt: null,
  loadTasks: async () => {
    const tasks = await getTasks();
    const { hiddenTaskIds, clearedAt } = get();
    const filteredTasks = tasks.filter(
      (task) =>
        !hiddenTaskIds.includes(task.id) && isAfterClearedAt(task, clearedAt)
    );
    set({ tasks: filteredTasks, currentTask: pickRunningTask(filteredTasks) });
  },
  loadTaskLogs: async (taskId) => {
    const logs = await getTaskLogs(taskId);
    set((state) => ({ logs: { ...state.logs, [taskId]: logs } }));
  },
  cancelTask: async (taskId) => {
    // 乐观更新：先立即更新前端状态，提供即时反馈
    set((state) => ({
      tasks: state.tasks.map((task) =>
        task.id === taskId
          ? { ...task, status: "Canceled", progress: 100, step: "canceled" }
          : task
      ),
      currentTask: pickRunningTask(
        state.tasks.map((task) =>
          task.id === taskId
            ? { ...task, status: "Canceled", progress: 100, step: "canceled" }
            : task
        )
      )
    }));

    // 然后异步调用后端 API
    try {
      await cancelTaskApi(taskId);
    } catch (error) {
      console.error("Failed to cancel task on backend:", error);
      // 前端已经显示为取消状态，后端失败不影响用户体验
    }
  },
  removeTask: async (taskId) => {
    // 保存当前状态以便失败时回滚
    const previousState = {
      tasks: get().tasks,
      logs: get().logs,
      hiddenTaskIds: get().hiddenTaskIds,
      currentTask: get().currentTask
    };

    // 乐观更新：先立即更新前端状态，提供即时反馈
    set((state) => {
      const nextTasks = state.tasks.filter((task) => task.id !== taskId);
      const { [taskId]: _, ...restLogs } = state.logs;
      return {
        tasks: nextTasks,
        logs: restLogs,
        hiddenTaskIds: [...state.hiddenTaskIds, taskId],
        currentTask: pickRunningTask(nextTasks)
      };
    });

    // 然后异步调用后端 API
    try {
      await deleteTaskApi(taskId);
    } catch (error) {
      console.error("Failed to delete task from backend:", error);
      // 后端删除失败，回滚到之前的状态
      set({
        tasks: previousState.tasks,
        logs: previousState.logs,
        hiddenTaskIds: previousState.hiddenTaskIds,
        currentTask: previousState.currentTask
      });
    }
  },
  clearTasks: async () => {
    // 保存当前状态以便失败时回滚
    const previousState = {
      tasks: get().tasks,
      logs: get().logs,
      hiddenTaskIds: get().hiddenTaskIds,
      clearedAt: get().clearedAt,
      currentTask: get().currentTask
    };

    // 乐观更新：先立即更新前端状态，提供即时反馈
    set((state) => ({
      tasks: state.tasks.filter((task) => task.status === "Running"),
      logs: {},
      currentTask: pickRunningTask(
        state.tasks.filter((task) => task.status === "Running")
      ),
      hiddenTaskIds: [
        ...new Set([...state.hiddenTaskIds, ...state.tasks.map((task) => task.id)])
      ],
      clearedAt: new Date().toISOString()
    }));

    // 然后异步调用后端 API
    try {
      await clearTasksApi();
    } catch (error) {
      console.error("Failed to clear tasks on backend:", error);
      // 后端清空失败，回滚到之前的状态
      set({
        tasks: previousState.tasks,
        logs: previousState.logs,
        hiddenTaskIds: previousState.hiddenTaskIds,
        clearedAt: previousState.clearedAt,
        currentTask: previousState.currentTask
      });
    }
  },
  addTask: (task) =>
    set((state) => {
      const nextTasks = [...state.tasks, task];
      return {
        tasks: nextTasks,
        currentTask: pickRunningTask(nextTasks)
      };
    }),
  updateTask: (taskId, updates) =>
    set((state) => {
      const nextTasks = state.tasks.map((task) =>
        task.id === taskId ? { ...task, ...updates } : task
      );
      return {
        tasks: nextTasks,
        currentTask: pickRunningTask(nextTasks)
      };
    })
}));
