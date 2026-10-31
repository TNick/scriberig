/** @file Provides a provider-neutral agent prompt composer. */

import type { FormEvent, ReactElement } from "react";
import type { AgentSelection } from "@scriberig/model";

/** Renders a prompt composer that submits only public semantic inputs. */
export function AgentComposer({
  selection,
  value,
  onChange,
  onSubmit,
  disabled = false,
}: {
  selection?: AgentSelection;
  value: string;
  onChange: (value: string) => void;
  onSubmit: (prompt: string) => void;
  disabled?: boolean;
}): ReactElement {
  const submit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const prompt = value.trim();
    if (prompt) onSubmit(prompt);
  };
  return (
    <form onSubmit={submit} aria-label="Agent composer">
      <label>
        Prompt
        <span style={{ position: "relative", display: "block" }}>
          <textarea
            value={value}
            disabled={disabled}
            onChange={(event) => onChange(event.currentTarget.value)}
            rows={4}
            style={{ width: "100%", paddingRight: "7rem", boxSizing: "border-box" }}
          />
          <button
            type="submit"
            title="Send prompt"
            aria-label="Send prompt"
            disabled={disabled || !value.trim()}
            style={{ position: "absolute", right: "0.5rem", bottom: "0.5rem" }}
          >
            Start agent
          </button>
        </span>
      </label>
      {selection?.entityId ? <p role="status">Selection: {selection.entityId}</p> : null}
    </form>
  );
}
