import { HashRouter, Navigate, Route } from "@solidjs/router";
import { render } from "solid-js/web";
import { App } from "./App";
import { initTheme } from "./theme";
import "./styles.css";

// Set the chosen theme on <html> before the first paint, so there's no flash of the OS default.
initTheme();

// Hash routing survives a reload under the Tauri asset protocol; `#/d/:id` picks the dashboard.
render(
  () => (
    <HashRouter>
      <Route path={["/", "/d/:id", "/settings"]} component={App} />
      <Route path="*" component={() => <Navigate href="/" />} />
    </HashRouter>
  ),
  document.getElementById("root")!,
);
