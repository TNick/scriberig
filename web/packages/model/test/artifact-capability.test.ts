/** @file Verifies artifact fidelity policy in the shared browser model. */
import { describe, expect, it } from "vitest";
import { artifactCapabilityAllowsEdit, newSessionId } from "../src";

describe("artifact model policies", () => {
  it("allows edits only for explicitly editable capabilities", () => {
    expect(artifactCapabilityAllowsEdit(undefined)).toBe(false);
    expect(artifactCapabilityAllowsEdit("read_only_derived")).toBe(false);
    expect(artifactCapabilityAllowsEdit("restricted_external_provider_edit")).toBe(false);
    expect(artifactCapabilityAllowsEdit("editable_with_opaque_preservation")).toBe(true);
  });

  it("creates Rust-compatible random session entity identifiers", () => {
    expect(newSessionId()).toMatch(/^ent_[a-f0-9]{32}$/);
  });
});
