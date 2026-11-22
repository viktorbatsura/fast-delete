import { useCallback, useEffect, useRef, useState } from "react";
import { backend } from "../api/backend";
import type { DirListing, DriveInfo } from "../api/types";
import { logDev, splitPath } from "../utils/format";

export function useNavigator() {
  const [drives, setDrives] = useState<DriveInfo[]>([]);
  const [path, setPath] = useState<string | null>(null);
  const [listing, setListing] = useState<DirListing | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
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

  const open = useCallback(async (target: string) => {
    const id = ++requestId.current;
    setLoading(true);
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

  useEffect(() => {
    refreshDrives().then((list) => {
      const start = list.find((d) => d.kind === "fixed") ?? list[0];
      if (start) open(start.path);
    });
  }, [refreshDrives, open]);

  return { drives, path, listing, error, loading, open, up, refreshDrives };
}
