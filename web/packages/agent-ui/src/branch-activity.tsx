/** @file Renders bounded durable branch activity summaries. */

import type { BranchActivityPage } from "@scriberig/protocol";
import type { ReactElement } from "react";

/** Renders durable branch activity without treating streams as history.
 * @param props Activity view properties.
 * @param props.page Bounded activity page to render.
 * @param props.onLoadMore Called when another page is requested.
 * @returns The rendered activity list.
 */
export function BranchActivityView({
  page,
  onLoadMore,
  onNavigate,
}: {
  page: BranchActivityPage;
  onLoadMore?: (cursor: string) => void;
  onNavigate?: (handle: string) => void;
}): ReactElement {
  return (
    <section aria-label={`Branch activity ${page.branchId}`}>
      <h2>Branch activity</h2>
      <ol>
        {page.items.map((item) => (
          <li key={item.id}>
            <span>{item.kind}: </span>
            <span>{item.summary}</span>
            {item.handles.length > 0 ? (
              <small>
                {item.handles.map((handle) => (
                  <button key={handle} type="button" onClick={() => onNavigate?.(handle)}>
                    Open {handle}
                  </button>
                ))}
              </small>
            ) : null}
          </li>
        ))}
      </ol>
      {page.nextCursor && onLoadMore ? (
        <button type="button" onClick={() => onLoadMore(page.nextCursor ?? "")}>
          Load more activity
        </button>
      ) : null}
    </section>
  );
}
