import { useCallback, useEffect, useRef, useState } from "react";
import { backend } from "../api/backend";
import type { DirListing, DriveInfo } from "../api/types";
import { logDev, splitPath } from "../utils/format";

const delay = (ms: number) => new Promise<void>((resolve) => window.setTimeout(resolve, ms));

export function useNavigator() {
  const [drives, setDrives] = useState<DriveInfo[]>([]);
  const [path, setPath] = useState<string | null>(null);
  const [listing, setListing] = useState<DirListing | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [refreshing, setRefreshing] = useState(false);
  // Only the latest request may update the state; slower earlier answers are dropped.
  const requestId = useRef(0);

  const refreshDrives = useCallback(async () => {
    try {
      const list = await backend.listDrives();
      setDrives(list);
      return list;
    } catch (e) {
      logDev(e);
      return [];
    }
  }, []);

  /** `quiet` reloads without dimming the list, for background refreshes of the same folder. */
  const open = useCallback(async (target: string, quiet = false) => {
    const id = ++requestId.current;
    if (!quiet) setLoading(true);
    setPath(target);
    try {
      const result = await backend.listDir(target);
      if (id !== requestId.current) return;
      setListing(result);
      setPath(result.path);
      setError(null);
    } catch (e) {
      if (id !== requestId.current) return;
      setListing(null);
      setError(String(e));
    } finally {
      if (id === requestId.current) setLoading(false);
    }
  }, []);

  const up = useCallback(() => {
    if (listing?.parent) {
      open(listing.parent);
      return;
    }
    // After an error there is no listing: derive the parent from the path itself.
    const crumbs = path ? splitPath(path) : [];
    if (crumbs.length > 1) open(crumbs[crumbs.length - 2].path);
  }, [listing, path, open]);

  /** Reloads free space and the current folder. */
  const refresh = useCallback(async () => {
    setRefreshing(true);
    // A minimum duration keeps the spinning icon readable as feedback even when the reload is instant.
    await Promise.all([refreshDrives(), path ? open(path, true) : Promise.resolve(), delay(450)]);
    setRefreshing(false);
  }, [refreshDrives, open, path]);

  useEffect(() => {
    refreshDrives().then((list) => {
      const start = list.find((d) => d.kind === "fixed") ?? list[0];
      if (start) open(start.path);
    });
  }, [refreshDrives, open]);

  return { drives, path, listing, error, loading, refreshing, open, up, refresh, refreshDrives };
}
