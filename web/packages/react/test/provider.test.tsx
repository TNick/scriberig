/** @file Verifies React provider accepts injected runtime objects. */
import { describe, expect, it } from "vitest";
import { createElement } from "react";
import { ScriberigProvider } from "../src/provider";

describe("ScriberigProvider", () => {
  it("creates a provider element from injected objects", () => {
    const value = { client: {} as never, model: {} as never, selection: {} as never, commands: [] };
    const element = createElement(ScriberigProvider, { value, children: "content" });
    expect(element.type).toBe(ScriberigProvider);
    expect(element.props.value).toBe(value);
  });
});
