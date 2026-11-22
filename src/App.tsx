// Temporary bridge check until the real UI is designed: shows that every layer is wired up.
import { useEffect, useState } from "react";
import { backend } from "./api/backend";
import type { DriveInfo, SystemInfo } from "./api/types";

const GB = 1024 ** 3;

export default function App() {
  const [info, setInfo] = useState<SystemInfo | null>(null);
  const [drives, setDrives] = useState<DriveInfo[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    Promise.all([backend.systemInfo(), backend.listDrives()])
      .then(([systemInfo, driveList]) => {
        setInfo(systemInfo);
        setDrives(driveList);
      })
      .catch((e) => setError(String(e)));
  }, []);

  return (
    <main className="hello">
      <h1>Fast Delete</h1>
      {info && (
        <p>
          v{info.appVersion} · {info.logicalCpus} logical CPUs · {info.defaultThreads} worker threads
        </p>
      )}
      <ul className="drives">
        {drives.map((d) => (
          <li key={d.letter}>
            <strong>{d.path}</strong> {d.label ?? d.kind} · {d.fileSystem ?? "—"}
            {d.totalBytes !== null && d.freeBytes !== null && (
              <span>
                {" "}
                · {(d.freeBytes / GB).toFixed(0)} of {(d.totalBytes / GB).toFixed(0)} GB free
              </span>
            )}
          </li>
        ))}
      </ul>
      {error && <p className="error">Rust bridge error: {error}</p>}
    </main>
  );
}
