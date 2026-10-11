/** @file Verifies reusable package dependency direction. */
import { describe, expect, it } from "vitest";
import { packageText } from "./test-helpers";
describe("package boundaries", () => {
  it("keeps lower packages free of MUI", () => {
    for (const p of ["protocol", "client", "model", "editor-core", "react", "components"]) {
      expect(packageText(p)).not.toContain("@mui/");
    }
  });
});
