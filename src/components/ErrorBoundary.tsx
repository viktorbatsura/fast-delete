import { Component, type ErrorInfo, type ReactNode } from "react";
import { logDev } from "../utils/format";

interface State {
  failed: boolean;
}

/** Last line of defence: a rendering bug shows a recovery screen instead of a blank window. */
export class ErrorBoundary extends Component<{ children: ReactNode }, State> {
  state: State = { failed: false };

  static getDerivedStateFromError(): State {
    return { failed: true };
  }

  componentDidCatch(error: Error, info: ErrorInfo): void {
    logDev({ error, info });
  }

  render() {
    if (!this.state.failed) return this.props.children;
    return (
      <div style={{ height: "100%", display: "grid", placeItems: "center", textAlign: "center", padding: 24 }}>
        <div style={{ display: "flex", flexDirection: "column", gap: 12, alignItems: "center" }}>
          <div style={{ fontSize: 20, fontWeight: 600 }}>Something went wrong in the interface.</div>
          <div style={{ color: "var(--text-2)" }}>Running deletions continue in the background.</div>
          <button
            className="press"
            style={{ height: 36, padding: "0 16px", borderRadius: 4, background: "var(--control)", border: "1px solid var(--control-stroke)" }}
            onClick={() => window.location.reload()}
          >
            Reload window
          </button>
        </div>
      </div>
    );
  }
}
