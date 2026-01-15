import { useCallback, useEffect } from "react";
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
  useEffect(() => {
    const shouldLoad =
      manager &&
      (scope !== "local" || (directory && directory.trim().length > 0));

    if (shouldLoad && manager) {
      loadPackages(manager, scope, directory, true);
    } else {
      clearPackages();
    }
  }, [manager, scope, directory, loadPackages, clearPackages]);

  return { packages, loading, error, loadPackages };
}
