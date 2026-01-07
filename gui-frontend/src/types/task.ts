export type JobStatus = "Pending" | "Running" | "Succeeded" | "Failed" | "Canceled";
export type Operation = "Install" | "Update" | "Uninstall";

export type Job = {
  id: string;
  manager: string;
  operation: Operation;
  target: string;
  status: JobStatus;
  progress?: number | null;
  step?: string | null;
  started_at?: string | null;
  finished_at?: string | null;
  logs: string[];
  error?: string | null;
};
