import type { Job, ManagerStatus, Package } from "../types";

export const mockManagers: ManagerStatus[] = [
  {
    name: "brew",
    version: "",
    available: true,
    package_count: 124,
    outdated_count: 6
  },
  {
    name: "npm",
    version: "",
    available: true,
    package_count: 256,
    outdated_count: 12
  },
  {
    name: "pnpm",
    version: "",
    available: true,
    package_count: 89,
    outdated_count: 1
  },
  {
    name: "yarn",
    version: "",
    available: false,
    package_count: 0,
    outdated_count: 0
  },
  {
    name: "pip",
    version: "",
    available: true,
    package_count: 78,
    outdated_count: 4
  }
];

export const mockPackages: Package[] = [
  {
    name: "node",
    version: "20.10.0",
    manager: "brew",
    description: "JavaScript runtime",
    size: 52428800,
    outdated: true,
    latest_version: "20.11.0"
  },
  {
    name: "npm",
    version: "10.2.3",
    manager: "npm",
    description: "Node package manager",
    size: 3145728,
    outdated: false
  },
  {
    name: "pnpm",
    version: "8.15.0",
    manager: "pnpm",
    description: "Fast package manager",
    size: 6291456,
    outdated: false
  },
  {
    name: "python",
    version: "3.11.7",
    manager: "brew",
    description: "Python language",
    size: 73400320,
    outdated: false
  }
];

let mockTasks: Job[] = [
  {
    id: "task-1",
    manager: "brew",
    operation: "Update",
    target: "node",
    status: "Running",
    logs: ["Downloading..."],
    started_at: new Date().toISOString()
  }
];

export const getMockTasks = (): Job[] => mockTasks;

export const removeMockTask = (taskId: string): void => {
  mockTasks = mockTasks.filter((task) => task.id !== taskId);
};

export const clearMockTasks = (): void => {
  mockTasks = [];
};
