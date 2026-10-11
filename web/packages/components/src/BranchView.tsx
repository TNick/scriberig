/** @file Renders one structured session branch summary. */
import type { ReactElement } from "react";
import type { BranchSummary } from "./component-types";

/** Displays branch metadata and nested branch links. */
export function BranchView({
  branch,
  className,
}: {
  branch: BranchSummary;
  className?: string;
}): ReactElement {
  return (
    <article className={className} aria-label={`Branch ${branch.title}`}>
      <h2>{branch.title}</h2>
      {branch.status ? <p>Status: {branch.status}</p> : null}
      {branch.children?.length ? <p>{branch.children.length} child branches</p> : null}
    </article>
  );
}
