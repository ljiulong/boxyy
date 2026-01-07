import { useEffect } from "react";
import { useTaskStore } from "../store/taskStore";

export function useTasks() {
  const { tasks, currentTask, loadTasks, cancelTask, removeTask, clearTasks } =
    useTaskStore();

  useEffect(() => {
    loadTasks();
  }, [loadTasks]);

  return { tasks, currentTask, cancelTask, removeTask, clearTasks };
}
