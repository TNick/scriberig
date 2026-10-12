/** @file Renders bounded durable session branches and result-first children. */

import { useMemo, useState, type ReactElement } from "react";
import type { BranchDescriptor } from "@scriberig/model";

/** Displays a keyboard-operable branch tree without loading activity transcripts. */
export function SessionBranchTree({
  branches,
  onSelect,
  onNavigate,
}: {
  branches: readonly BranchDescriptor[];
  onSelect?: (branch: BranchDescriptor) => void;
  onNavigate?: (handle: string) => void;
}): ReactElement {
  const [expanded, setExpanded] = useState<Readonly<Record<string, boolean>>>({});
  const children = useMemo(() => {
    const grouped = new Map<string, BranchDescriptor[]>();
    for (const branch of branches) {
      const parent = branch.parentBranchId ?? "";
      const values = grouped.get(parent) ?? [];
      values.push(branch);
      grouped.set(parent, values);
    }
    return grouped;
  }, [branches]);
  const render = (branch: BranchDescriptor, depth: number): ReactElement => {
    const nested = children.get(branch.branchId) ?? [];
    const isExpanded = expanded[branch.branchId] ?? false;
    return (
      <li key={branch.branchId}>
        <div style={{ paddingInlineStart: `${depth}rem` }}>
          {nested.length > 0 ? (
            <button
              type="button"
              aria-label={`${isExpanded ? "Collapse" : "Expand"} ${branch.name}`}
              aria-expanded={isExpanded}
              onClick={() => setExpanded((value) => ({ ...value, [branch.branchId]: !isExpanded }))}
            >
              {isExpanded ? "−" : "+"}
            </button>
          ) : null}
          <button type="button" onClick={() => onSelect?.(branch)}>
            {branch.name} ({branch.status})
          </button>
          {branch.result ? (
            <span>
              {" "}
              — {branch.result.summary ?? "completed"}{" "}
              <button type="button" onClick={() => onNavigate?.(branch.result?.resultId ?? "")}>
                Open result {branch.result.resultId}
              </button>
              {branch.result.handles.map((handle) => (
                <button key={handle} type="button" onClick={() => onNavigate?.(handle)}>
                  Open {handle}
                </button>
              ))}
            </span>
          ) : null}
        </div>
        {isExpanded && nested.length > 0 ? (
          <ul>{nested.map((child) => render(child, depth + 1))}</ul>
        ) : null}
      </li>
    );
  };
  const roots = children.get("") ?? branches;
  return <ul aria-label="Agent branches">{roots.map((branch) => render(branch, 0))}</ul>;
}
