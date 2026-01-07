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
    await cancelTaskApi(taskId);
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
  },
  removeTask: async (taskId) => {
    await deleteTaskApi(taskId);
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
  },
  clearTasks: async () => {
    await clearTasksApi();
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
