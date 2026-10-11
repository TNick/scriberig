/** @file Renders a paged semantic relation. */
import type { ReactElement } from "react";
import type { ObjectSummary } from "@scriberig/protocol";

/** Renders relation members and delegates selection and paging. */
export function RelationBrowser({
  relation,
  items,
  nextCursor,
  onSelect,
  onLoadMore,
  className,
}: {
  relation: string;
  items: readonly ObjectSummary[];
  nextCursor?: string;
  onSelect?: (item: ObjectSummary) => void;
  onLoadMore?: () => void;
  className?: string;
}): ReactElement {
  return (
    <section className={className} aria-label={relation}>
      <h2>{relation}</h2>
      <ul>
        {items.map((item) => (
          <li key={item.ref.entityId}>
            <button type="button" onClick={() => onSelect?.(item)}>
              {item.label ?? item.ref.entityId}
            </button>
          </li>
        ))}
      </ul>
      {nextCursor ? (
        <button type="button" onClick={onLoadMore}>
          Load more
        </button>
      ) : null}
    </section>
  );
}
