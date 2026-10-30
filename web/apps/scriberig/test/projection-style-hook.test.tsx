/** @file Verifies the integrated app receives shared projection-style controls through React. */

import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { SelectionController } from "@scriberig/editor-core";
import { WorkspaceModel } from "@scriberig/model";
import { ScriberigProvider, useProjectionStyleController } from "@scriberig/react";

/** Renders whether each shared style operation is available to React consumers.
 * @returns A compact static marker used by the hook test.
 */
function ProjectionStyleProbe() {
  const controller = useProjectionStyleController();
  return (
    <output>
      {[controller.schema, controller.resolve, controller.set].every(
        (operation) => typeof operation === "function",
      )
        ? "style operations available"
        : "style operations missing"}
    </output>
  );
}

describe("useProjectionStyleController", () => {
  it("exposes the model-owned style controller through the shared provider", () => {
    const html = renderToStaticMarkup(
      <ScriberigProvider
        value={{
          client: {} as never,
          model: new WorkspaceModel(),
          selection: new SelectionController(),
          commands: [],
        }}
      >
        <ProjectionStyleProbe />
      </ScriberigProvider>,
    );

    expect(html).toContain("style operations available");
  });
});
