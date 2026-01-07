import { create } from "zustand";
import type { ManagerStatus } from "../types";
import { scanManagers, refreshManager as refreshManagerApi } from "../lib/api";

type ManagerState = {
  managers: ManagerStatus[];
  selectedManager: string | null;
  loading: boolean;
  refreshing: boolean;
  error: string | null;
  packageScope: "global" | "local";
  packageDirectory: string;
  loadManagers: () => Promise<void>;
  refreshManager: (manager: string, scope?: string, directory?: string) => Promise<void>;
  refreshAll: () => Promise<void>;
  selectManager: (manager: string | null) => void;
  setPackageScope: (scope: "global" | "local") => void;
  setPackageDirectory: (directory: string) => void;
};

export const useManagerStore = create<ManagerState>((set, get) => ({
  managers: [],
  selectedManager: null,
  loading: false,
  refreshing: false,
  error: null,
  packageScope: "global",
  packageDirectory: "",
  loadManagers: async () => {
    set({ loading: true, error: null });
    try {
      const managers = await scanManagers();
      set({ managers, loading: false });
      if (!get().selectedManager && managers.length > 0) {
        set({ selectedManager: managers[0].name });
      }
    } catch (error) {
      set({ error: getErrorMessage(error), loading: false });
    }
  },
  refreshManager: async (manager, scope, directory) => {
    try {
      await refreshManagerApi(manager, scope, directory);
      await get().loadManagers();
    } catch (error) {
      set({ error: getErrorMessage(error) });
      throw error;
    }
  },
  refreshAll: async () => {
    const { managers, refreshing } = get();
    if (refreshing) {
      return;
    }
    set({ refreshing: true });
    try {
      await Promise.all(
        managers.map((manager) => refreshManagerApi(manager.name))
      );
      await get().loadManagers();
    } catch (error) {
      set({ error: getErrorMessage(error) });
      throw error;
    } finally {
      set({ refreshing: false });
    }
  },
  selectManager: (manager) => set({ selectedManager: manager }),
  setPackageScope: (scope) => set({ packageScope: scope }),
  setPackageDirectory: (directory) => set({ packageDirectory: directory })
}));

function getErrorMessage(error: unknown): string {
  if (error instanceof Error) {
    return error.message;
  }
  if (typeof error === "string") {
    return error;
  }
  try {
    return JSON.stringify(error);
  } catch {
    return "Unknown error";
  }
}
