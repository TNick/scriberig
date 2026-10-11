/** @file Rejects private cross-package source imports. */
import { describe, expect, it } from "vitest";
import { packageText } from "./test-helpers";
describe("public imports", () => {
  it("does not deep import package source paths", () => {
    for (const p of [
      "protocol",
      "client",
      "model",
      "editor-core",
      "react",
      "components",
      "components-mui",
    ]) {
      expect(packageText(p)).not.toMatch(/@scriberig\/[^"']+\/src\//);
    }
  });
});
