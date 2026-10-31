/** @file Defines an optional Scriberig Material UI theme. */
import { createTheme, type ThemeOptions } from "@mui/material/styles";

/** Creates an optional semantic theme for Scriberig consumers.
 * @param options Optional host theme overrides.
 * @returns A MUI theme instance.
 */
export function createScriberigTheme(options?: ThemeOptions) {
  const dark = options?.palette?.mode === "dark";
  const primary = dark
    ? { main: "#75c7bc", dark: "#4ca89c", light: "#385d5a", contrastText: "#112321" }
    : { main: "#176b67", dark: "#10514e", light: "#dcefeb", contrastText: "#ffffff" };
  const secondary = dark
    ? { main: "#e6b66b", dark: "#c88a32", light: "#5b472d", contrastText: "#202c32" }
    : { main: "#c88a32", dark: "#90601d", light: "#f4e8d4", contrastText: "#202c32" };
  const background = dark
    ? { default: "#293336", paper: "#323e41" }
    : { default: "#f7f8f5", paper: "#ffffff" };
  const text = dark
    ? { primary: "#f0f4f2", secondary: "#c1ceca" }
    : { primary: "#202c32", secondary: "#68777b" };
  return createTheme({
    ...options,
    palette: {
      ...options?.palette,
      mode: dark ? "dark" : "light",
      primary: { ...primary, ...options?.palette?.primary },
      secondary: { ...secondary, ...options?.palette?.secondary },
      background: { ...background, ...options?.palette?.background },
      text: { ...text, ...options?.palette?.text },
      divider: options?.palette?.divider ?? (dark ? "#4a585b" : "#dfe5e1"),
      success: { main: "#28745b" },
      warning: { main: "#a96d1d" },
      error: { main: "#b4473d" },
    },
    shape: { borderRadius: 10, ...options?.shape },
    typography: {
      fontFamily: '"Inter", "Segoe UI", sans-serif',
      h1: { fontWeight: 720, letterSpacing: "-0.045em" },
      h2: { fontWeight: 720, letterSpacing: "-0.04em" },
      h3: { fontWeight: 700, letterSpacing: "-0.035em" },
      h4: { fontWeight: 700, letterSpacing: "-0.03em", fontSize: "1.8rem" },
      h5: { fontWeight: 680, letterSpacing: "-0.025em" },
      button: { textTransform: "none", fontWeight: 650 },
      ...options?.typography,
    },
    components: {
      MuiButton: { styleOverrides: { root: { borderRadius: 8, minHeight: 40 } } },
      MuiPaper: { styleOverrides: { rounded: { backgroundImage: "none" } } },
      MuiOutlinedInput: {
        styleOverrides: {
          root: { backgroundColor: dark ? "#323e41" : "#ffffff" },
        },
      },
      MuiCssBaseline: {
        styleOverrides: {
          body: { margin: 0 },
          "*": { boxSizing: "border-box" },
          "*:focus-visible": { outline: "3px solid #c88a32", outlineOffset: 2 },
        },
      },
      ...options?.components,
    },
  });
}
