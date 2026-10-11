/** @file Defines an optional Scriberig Material UI theme. */
import { createTheme, type ThemeOptions } from "@mui/material/styles";

/** Creates an optional semantic theme for Scriberig consumers.
 * @param options Optional host theme overrides.
 * @returns A MUI theme instance.
 */
export function createScriberigTheme(options?: ThemeOptions) {
  return createTheme({
    palette: {
      success: { main: "#2e7d32" },
      warning: { main: "#ed6c02" },
      error: { main: "#d32f2f" },
      ...options?.palette,
    },
    ...options,
  });
}
