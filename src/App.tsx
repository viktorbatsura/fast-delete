import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

type SystemInfo = {
  appVersion: string;
  logicalCpus: number;
};

export default function App() {
  const [info, setInfo] = useState<SystemInfo | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    invoke<SystemInfo>("system_info")
      .then(setInfo)
      .catch((e) => setError(String(e)));
  }, []);

  return (
    <main className="hello">
      <h1>Fast Delete</h1>
      {info && (
        <p>
          v{info.appVersion} · {info.logicalCpus} logical CPUs available
        </p>
      )}
      {error && <p className="error">Rust bridge error: {error}</p>}
    </main>
  );
}
