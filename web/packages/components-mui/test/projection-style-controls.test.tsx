/** @file Verifies shared projection-style metadata and preference affordances. */

import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import { MuiProjectionStyleControls } from "../src";

describe("MuiProjectionStyleControls", () => {
  it("renders adapter metadata and forwards opaque preference intent", () => {
    const markup = renderToStaticMarkup(
      <MuiProjectionStyleControls
        schema={{
          reference: { adapterId: "rust-source-style", version: "1" },
          projectionKinds: ["rust"],
          properties: [
            {
              key: "indent",
              label: "Indent",
              kind: "enum",
              allowedValues: ["spaces", "tabs"],
              default: [115, 112, 97, 99, 101, 115],
              applicability: "both",
              affectsFidelity: false,
              requiredForMaterialization: false,
            },
          ],
        }}
        resolution={{
          purpose: "interactiveView",
          reference: { kind: "inline" },
          value: [105, 110, 100, 101, 110, 116, 61, 116, 97, 98, 115],
          applied: [
            {
              kind: "user-preference",
              identity: "local-user",
              value: [105, 110, 100, 101, 110, 116, 61, 116, 97, 98, 115],
            },
          ],
          fidelity: { fidelity: "losslessEditable", omittedCategories: [] },
        }}
        preference="indent=tabs"
        onPreferenceChange={vi.fn()}
        onSave={vi.fn()}
        onClear={vi.fn()}
      />,
    );

    expect(markup).toContain("rust-source-style");
    expect(markup).toContain("losslessEditable");
    expect(markup).toContain("user-preference: local-user");
    expect(markup).toContain("Indent (enum) · allowed: spaces, tabs");
    expect(markup).toContain("Save personal style");
    expect(markup).toContain("Clear personal style");
    expect(markup).toContain("indent=tabs");
  });
});
