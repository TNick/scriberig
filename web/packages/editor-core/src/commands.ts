/** @file Defines discoverable headless command descriptors. */
import type { Selection } from "./selection";
/** Describes a capability-aware semantic command. */
export interface CommandDescriptor {
  id: string;
  title: string;
  mutating: boolean;
  requiredCapability?: string;
  isApplicable?: (selection?: Selection) => boolean;
  execute: string;
}
/** Discovers commands accepted by current selection and capabilities. @param commands Registered commands. @param selection Current selection. @param capabilities Available capability IDs. @returns Applicable commands. */
export function discoverCommands(
  commands: readonly CommandDescriptor[],
  selection: Selection | undefined,
  capabilities: ReadonlySet<string>,
): CommandDescriptor[] {
  return commands.filter(
    (command) =>
      (!command.requiredCapability || capabilities.has(command.requiredCapability)) &&
      (!command.isApplicable || command.isApplicable(selection)),
  );
}
