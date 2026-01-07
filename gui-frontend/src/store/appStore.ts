import { create } from "zustand";

type View = "dashboard" | "manager" | "tasks" | "settings";

type AppState = {
  theme: "light" | "dark" | "system";
  locale: "zh" | "en";
  sidebarCollapsed: boolean;
  currentView: View;
  searchQuery: string;
  setTheme: (theme: AppState["theme"]) => void;
  setLocale: (locale: AppState["locale"]) => void;
  setSidebarCollapsed: (collapsed: boolean) => void;
  setCurrentView: (view: View) => void;
  setSearchQuery: (query: string) => void;
};

export const useAppStore = create<AppState>((set) => ({
  theme: "system",
  locale: "zh",
  sidebarCollapsed: false,
  currentView: "dashboard",
  searchQuery: "",
  setTheme: (theme) => set({ theme }),
  setLocale: (locale) => set({ locale }),
  setSidebarCollapsed: (collapsed) => set({ sidebarCollapsed: collapsed }),
  setCurrentView: (view) => set({ currentView: view }),
  setSearchQuery: (query) => set({ searchQuery: query })
}));
