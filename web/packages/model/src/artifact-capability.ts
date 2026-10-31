/** @file Defines model policy for editing artifacts with declared fidelity. */
import type { ArtifactEditCapability } from "@scriberig/protocol";

/** Reports whether a selected artifact capability permits semantic candidates.
 * @param capability Capability attached to the selected artifact snapshot.
 * @returns True only for capabilities that explicitly authorize editing.
 */
export function artifactCapabilityAllowsEdit(
  capability: ArtifactEditCapability | null | undefined,
): boolean {
  return (
    capability === "lossless_editable" ||
    capability === "loss_aware_editable" ||
    capability === "editable_with_opaque_preservation"
  );
}
