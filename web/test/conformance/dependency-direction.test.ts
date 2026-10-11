/** @file Verifies MUI dependency isolation. */
import { expect, it } from "vitest";
import { packageText } from "./test-helpers";
it("allows MUI only in its package boundary", () => {
  expect(packageText("components-mui")).toContain("@mui/");
  expect(packageText("components")).not.toContain("@mui/");
});
