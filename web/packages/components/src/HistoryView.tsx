/** @file Renders semantic history and lineage entries. */
import type { ReactElement } from "react";
import type { DiffEntry } from "./component-types";

/** Renders selectable immutable history entries. */
export function HistoryView({
  entries,
  onSelect,
  className,
}: {
  entries: readonly DiffEntry[];
  onSelect?: (entry: DiffEntry) => void;
  className?: string;
}): ReactElement {
  return (
    <ol className={className} aria-label="History">
      {entries.map((entry) => (
        <li key={entry.id}>
          <button type="button" onClick={() => onSelect?.(entry)}>
            {entry.summary}
          </button>
        </li>
      ))}
    </ol>
  );
}
