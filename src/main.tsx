import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { ErrorBoundary } from "./components/ErrorBoundary";
import { loadSettings } from "./hooks/useSettings";
import { detectMica, resolveTheme } from "./theme/theme";
import "./styles/tokens.css";
import "./styles/global.css";

// Set the theme before the first paint so the window never flashes the wrong colors.
document.documentElement.dataset.theme = resolveTheme(loadSettings().theme);
void detectMica();

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <ErrorBoundary>
      <App />
    </ErrorBoundary>
  </React.StrictMode>,
);
