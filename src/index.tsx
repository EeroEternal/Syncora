/* @refresh reload */
import { render } from "solid-js/web";
import { Router, Route } from "@solidjs/router";
import App from "./App";
import Dashboard from "./pages/dashboard";
import Folders from "./pages/folders";
import Conflicts from "./pages/conflicts";
import Logs from "./pages/logs";
import Settings from "./pages/settings";
import Login from "./pages/login";
import "./index.css";

const root = document.getElementById("root");

render(
  () => (
    <Router>
      <Route path="/login" component={Login} />
      <Route path="/" component={App}>
        <Route path="/" component={Dashboard} />
        <Route path="/folders" component={Folders} />
        <Route path="/conflicts" component={Conflicts} />
        <Route path="/logs" component={Logs} />
        <Route path="/settings" component={Settings} />
      </Route>
    </Router>
  ),
  root!
);
