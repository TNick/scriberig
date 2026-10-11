/** @file Renders structured semantic diff entries without computing them. */
import type { ReactElement } from "react";
import type { DiffEntry } from "./component-types";

/** Renders caller-provided semantic diff data. */
export function SemanticDiffView({
  entries,
  renderEntry,
  className,
}: {
  entries: readonly DiffEntry[];
  renderEntry?: (entry: DiffEntry) => ReactElement;
  className?: string;
}): ReactElement {
  return (
    <ul className={className} aria-label="Semantic differences">
      {entries.map((entry) => (
        <li key={entry.id}>
          {renderEntry?.(entry) ?? (
            <>
              <strong>{entry.kind}</strong>: {entry.summary}
            </>
          )}
        </li>
      ))}
    </ul>
  );
}
