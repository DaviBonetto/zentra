import React from "react";
import ReactDOM from "react-dom/client";
import "./styles/globals.css";
import WindowRoot from "./window/WindowRoot";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <WindowRoot />
  </React.StrictMode>,
);
