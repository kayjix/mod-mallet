import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { loadApi } from "./api";
import { initialLang } from "./i18n";

const root = document.getElementById("root");
if (root === null) {
  throw new Error("index.html has no #root element");
}

loadApi().then((api) => {
  ReactDOM.createRoot(root).render(
    <React.StrictMode>
      <App api={api} startLang={initialLang()} />
    </React.StrictMode>,
  );
});
