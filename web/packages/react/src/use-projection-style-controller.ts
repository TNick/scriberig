/** @file Binds the shared projection-style model controller to React context. */

import { useMemo } from "react";
import { ProjectionStyleController } from "@scriberig/model";
import { useScriberig } from "./provider";

/** Returns the shared projection-style controller for the current runtime.
 * @returns Stable controller backed by the provider's semantic client.
 */
export function useProjectionStyleController(): ProjectionStyleController {
  const { client } = useScriberig();
  return useMemo(() => new ProjectionStyleController(client), [client]);
}
