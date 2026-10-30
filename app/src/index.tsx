import { HashRouter, Navigate, Route } from "@solidjs/router";
import { render } from "solid-js/web";
import { App } from "./App";
import "./styles.css";

// Hash routing survives a reload under the Tauri asset protocol; `#/d/:name` picks the dashboard.
render(
  () => (
    <HashRouter>
      <Route path={["/", "/d/:name", "/settings"]} component={App} />
      <Route path="*" component={() => <Navigate href="/" />} />
    </HashRouter>
  ),
  document.getElementById("root")!,
);
