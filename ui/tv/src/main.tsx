import React from "react";
import ReactDOM from "react-dom/client";
import { applyTheme, parseTheme } from "@om/shared";
import defaultTheme from "../../../themes/default/theme.json";
import App from "./App";
import "./index.css";

// Applied before the first render so no frame is drawn without colors. The default
// theme is checked in CI (ui/shared theme tests); failing loudly here beats an unreadable UI.
const theme = parseTheme(defaultTheme);
if (!theme.ok) throw new Error(`Thème par défaut rejeté :\n${theme.errors.join("\n")}`);
applyTheme(theme.theme);

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
