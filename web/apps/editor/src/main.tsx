/** @file Provides a second host entry for the integrated editor. */

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "@scriberig/app";

/** Mounts the shared public-package editor composition. */
/** @param element Root element that receives the application. */
export function mountEditor(element: HTMLElement): void {
  createRoot(element).render(
    <StrictMode>
      <App />
    </StrictMode>,
  );
}
