import { useCallback, useEffect, useRef } from "react";
import { usePackageStore } from "../store/packageStore";

export function usePackages(
  manager: string | null,
  scope: string,
  directory: string
) {
  const {
    packages,
    loading,
    error,
    loadPackages: loadPackagesRaw,
    clearPackages
  } = usePackageStore();
  const loadPackages = useCallback(
    (
      target: string,
      selectedScope: string,
      selectedDirectory: string,
      force?: boolean
    ) => loadPackagesRaw(target, selectedScope, selectedDirectory, force),
    [loadPackagesRaw]
  );
  const lastScopeRef = useRef<string | null>(null);
  const lastDirectoryRef = useRef<string | null>(null);

  useEffect(() => {
    const shouldLoad =
      manager &&
      (scope !== "local" || (directory && directory.trim().length > 0));

    if (shouldLoad && manager) {
      const scopeChanged = lastScopeRef.current !== scope;
      const dirChanged = lastDirectoryRef.current !== directory;
      const force = scopeChanged || dirChanged;
      lastScopeRef.current = scope;
      lastDirectoryRef.current = directory;
      loadPackages(manager, scope, directory, force);
    } else {
      clearPackages();
    }
  }, [manager, scope, directory, loadPackages, clearPackages]);

  return { packages, loading, error, loadPackages };
}
