export type Package = {
  name: string;
  version: string;
  manager: string;
  description?: string | null;
  homepage?: string | null;
  repository?: string | null;
  license?: string | null;
  installed_path?: string | null;
  size?: number | null;
  outdated: boolean;
  latest_version?: string | null;
};
