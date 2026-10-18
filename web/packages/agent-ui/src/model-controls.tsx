/** @file Renders shared model-profile and model-specific thinking controls. */

import type { ReactElement } from "react";
import type { ModelLibraryResponse, SessionExecutionSelection } from "@scriberig/protocol";
import type {
  AgentPresentationPreferences,
  AgentPresentationPreferenceLayer,
} from "@scriberig/model";

/** Renders model and thinking selectors from the shared model library.
 * @param props Shared model controls.
 * @param props.library Safe model descriptors and selectable profiles.
 * @param props.selection Authoritative session execution selection.
 * @param props.onModelChange Called with a selected profile identity.
 * @param props.onThinkingChange Called with a level advertised by the selected model.
 * @param props.disabled Whether controls are unavailable during a mutation.
 * @returns Accessible model and thinking selectors.
 */
export function AgentModelControls({
  library,
  selection,
  onModelChange,
  onThinkingChange,
  disabled = false,
}: {
  library: ModelLibraryResponse;
  selection: SessionExecutionSelection;
  onModelChange: (profileId: string) => void;
  onThinkingChange: (thinkingLevel: string) => void;
  disabled?: boolean;
}): ReactElement {
  const profiles = library.profiles.filter((profile) => profile.enabled);
  const profile = profiles.find((value) => value.id === selection.modelProfile);
  const model =
    profile === undefined ? undefined : library.models.find((value) => value.id === profile.model);
  return (
    <fieldset aria-label="Agent model controls" disabled={disabled}>
      <label>
        Model
        <select
          aria-label="Agent model"
          value={profile?.id ?? ""}
          onChange={(event) => onModelChange(event.currentTarget.value)}
        >
          <option value="" disabled>
            Select a model
          </option>
          {profiles.map((value) => (
            <option key={value.id} value={value.id}>
              {value.displayName}
            </option>
          ))}
        </select>
      </label>
      <label>
        Thinking level
        <select
          aria-label="Agent thinking level"
          value={selection.thinkingLevel ?? ""}
          disabled={model === undefined || model.thinkingLevels.length === 0}
          onChange={(event) => onThinkingChange(event.currentTarget.value)}
        >
          {model === undefined || model.thinkingLevels.length === 0 ? (
            <option value="">No thinking levels advertised</option>
          ) : null}
          {model?.thinkingLevels.map((level) => (
            <option key={level.id} value={level.id}>
              {level.label}
            </option>
          ))}
        </select>
      </label>
    </fieldset>
  );
}

/** Renders independent client-local controls for captured activity.
 * @param props Display controls and current preference values.
 * @param props.value Current presentation choices.
 * @param props.onChange Called with one changed presentation field.
 * @param props.disabled Whether the controls are unavailable.
 * @returns Accessible presentation controls.
 */
export function AgentPresentationControls({
  value,
  onChange,
  disabled = false,
}: {
  value: AgentPresentationPreferences;
  onChange: (value: AgentPresentationPreferenceLayer) => void;
  disabled?: boolean;
}): ReactElement {
  return (
    <fieldset aria-label="Agent view controls" disabled={disabled}>
      <label>
        Reasoning
        <select
          aria-label="Reasoning visibility"
          value={value.reasoning}
          onChange={(event) =>
            onChange({
              reasoning: event.currentTarget.value as AgentPresentationPreferences["reasoning"],
            })
          }
        >
          <option value="hidden">Hidden</option>
          <option value="shown">Shown</option>
        </select>
      </label>
      <label>
        Tool invocation
        <select
          aria-label="Tool invocation detail"
          value={value.toolInvocation}
          onChange={(event) =>
            onChange({
              toolInvocation: event.currentTarget
                .value as AgentPresentationPreferences["toolInvocation"],
            })
          }
        >
          <option value="simple">Simple</option>
          <option value="detailed">Detailed</option>
        </select>
      </label>
      <label>
        Tool output
        <select
          aria-label="Tool output visibility"
          value={value.toolOutput}
          onChange={(event) =>
            onChange({
              toolOutput: event.currentTarget.value as AgentPresentationPreferences["toolOutput"],
            })
          }
        >
          <option value="hidden">Hidden</option>
          <option value="summary">Summary</option>
          <option value="full">Full</option>
        </select>
      </label>
      <label>
        Prompt
        <select
          aria-label="Prompt visibility"
          value={value.prompt}
          onChange={(event) =>
            onChange({
              prompt: event.currentTarget.value as AgentPresentationPreferences["prompt"],
            })
          }
        >
          <option value="hidden">Hidden</option>
          <option value="shown">Shown</option>
        </select>
      </label>
    </fieldset>
  );
}
