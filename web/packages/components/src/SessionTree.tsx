/** @file Renders progressive session branch summaries. */
import type { ReactElement } from "react";
import type { BranchSummary } from "./component-types";

/** Renders branches without loading their transcripts. */
export function SessionTree({
  branches,
  onSelect,
  className,
}: {
  branches: readonly BranchSummary[];
  onSelect?: (branch: BranchSummary) => void;
  className?: string;
}): ReactElement {
  const render = (branch: BranchSummary): ReactElement => (
    <li key={branch.id}>
      <button type="button" onClick={() => onSelect?.(branch)}>
        {branch.title}
        {branch.status ? ` (${branch.status})` : ""}
      </button>
      {branch.children?.length ? <ul>{branch.children.map(render)}</ul> : null}
    </li>
  );
  return (
    <ul className={className} aria-label="Session branches">
      {branches.map(render)}
    </ul>
  );
}
