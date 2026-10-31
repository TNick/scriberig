/** @file Renders upstream model and model-specific thinking controls. */

import type { ReactElement } from "react";
import type { ModelLibraryResponse, SessionExecutionSelection } from "@scriberig/protocol";
import type {
  AgentPresentationPreferences,
  AgentPresentationPreferenceLayer,
} from "@scriberig/model";

/** Renders model and thinking selectors from the shared model library.
 * @param props Shared model controls.
 * @param props.library Safe model descriptors used as display data.
 * @param props.selection Authoritative session execution selection.
 * @param props.onModelChange Called with upstream provider and model ids.
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
  onModelChange: (providerId: string, modelId: string) => void;
  onThinkingChange: (thinkingLevel: string) => void;
  disabled?: boolean;
}): ReactElement {
  const models = library.models;
  const providers = Array.from(new Set(models.map((value) => value.providerId)));
  const model = models.find(
    (value) => value.providerId === selection.providerId && value.modelId === selection.modelId,
  );
  const providerModels = models.filter((value) => value.providerId === selection.providerId);
  return (
    <fieldset aria-label="Agent model controls" disabled={disabled}>
      <label>
        Provider
        <select
          aria-label="Agent provider"
          value={selection.providerId ?? ""}
          onChange={(event) => {
            const provider = event.currentTarget.value;
            const firstModel = models.find((value) => value.providerId === provider);
            if (firstModel !== undefined) onModelChange(firstModel.providerId, firstModel.modelId);
          }}
        >
          <option value="" disabled>
            Select a provider
          </option>
          {providers.map((provider) => (
            <option key={provider} value={provider}>
              {provider}
            </option>
          ))}
        </select>
      </label>
      <label>
        Model
        <select
          aria-label="Agent model"
          value={model === undefined ? "" : model.modelId}
          onChange={(event) => {
            const selected = providerModels.find(
              (value) => value.modelId === event.currentTarget.value,
            );
            if (selected !== undefined) onModelChange(selected.providerId, selected.modelId);
          }}
        >
          <option value="" disabled>
            Select a model
          </option>
          {providerModels.map((value) => (
            <option key={value.modelId} value={value.modelId}>
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
            <option key={level} value={level}>
              {formatThinkingLevel(level)}
            </option>
          ))}
        </select>
      </label>
    </fieldset>
  );
}

/**
 * Formats a scalar upstream thinking-level id for display.
 * @param level Canonical upstream thinking-level id.
 * @returns A human-readable label while preserving the underlying id as the value.
 */
function formatThinkingLevel(level: string): string {
  if (level.length === 0) {
    return level;
  }
  return `${level[0]?.toUpperCase() ?? ""}${level.slice(1).replaceAll("-", " ")}`;
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
