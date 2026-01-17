import { useEffect } from "react";
import { useManagerStore } from "../store/managerStore";

export function useManagers() {
  const {
    managers,
    loading,
    error,
    loadManagers,
    refreshManager,
    refreshAll
  } = useManagerStore();

  useEffect(() => {
    loadManagers();
  }, [loadManagers]);

  return { managers, loading, error, loadManagers, refreshManager, refreshAll };
}
