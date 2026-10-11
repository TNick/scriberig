/** @file Bootstraps the reference Scriberig browser application. */

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import { App } from "./App";

// Block: Resolve the browser mount point before creating the React root.
const mount = document.getElementById("root");
if (mount === null) {
  throw new Error("Missing #root mount element");
}

// Block: Create the React root separately so executable calls stay explicit and inspectable.
const root = createRoot(mount);

// Block: Mount the integrated editor shell without embedding reusable semantic behavior here.
root.render(
  <StrictMode>
    <App />
  </StrictMode>,
);
