import { create } from "zustand";
import type { Package } from "../types";
import { getManagerPackages } from "../lib/api";

type PackageFilter = "all" | "outdated";

type PackageState = {
  packages: Package[];
  selectedPackage: Package | null;
  filter: PackageFilter;
  loading: boolean;
  error: string | null;
  currentManager: string | null;
  requestId: number;
  cache: Record<string, { packages: Package[]; cachedAt: number }>;
  loadPackages: (
    manager: string,
    scope?: string,
    directory?: string,
    force?: boolean
  ) => Promise<void>;
  selectPackage: (pkg: Package | null) => void;
  setFilter: (filter: PackageFilter) => void;
  clearPackages: () => void;
};

export const usePackageStore = create<PackageState>((set, get) => ({
  packages: [],
  selectedPackage: null,
  filter: "all",
  loading: false,
  error: null,
  currentManager: null,
  requestId: 0,
  cache: {},
  loadPackages: async (
    manager: string,
    scope?: string,
    directory?: string,
    force?: boolean
  ) => {
    const cacheKey = `${scope ?? "global"}|${directory ?? ""}|${manager}`;
    const cacheEntry = get().cache[cacheKey];
    const now = Date.now();
    const cacheFresh = cacheEntry && now - cacheEntry.cachedAt < 2 * 60 * 1000;

    if (cacheEntry && !force) {
      set({
        packages: cacheEntry.packages,
        loading: !cacheFresh,
        error: null,
        currentManager: manager,
        selectedPackage: null
      });
      if (cacheFresh) {
        return;
      }
    }

    const nextRequestId = get().requestId + 1;
    set({
      loading: true,
      error: null,
      currentManager: manager,
      requestId: nextRequestId,
      packages: cacheEntry?.packages ?? [],
      selectedPackage: null
    });
    try {
      const packages = await getManagerPackages(manager, scope, directory, force);
      const state = get();
      if (state.requestId !== nextRequestId || state.currentManager !== manager) {
        set((current) => ({
          cache: {
            ...current.cache,
            [cacheKey]: { packages, cachedAt: Date.now() }
          }
        }));
        return;
      }
      set((current) => ({
        packages,
        loading: false,
        cache: {
          ...current.cache,
          [cacheKey]: { packages, cachedAt: Date.now() }
        }
      }));
    } catch (error) {
      const state = get();
      if (state.requestId !== nextRequestId || state.currentManager !== manager) {
        return;
      }
      set({ error: String(error), loading: false });
    }
  },
  selectPackage: (pkg) => set({ selectedPackage: pkg }),
  setFilter: (filter) => set({ filter }),
  clearPackages: () =>
    set({
      packages: [],
      selectedPackage: null,
      loading: false,
      error: null,
      currentManager: null
    })
}));
