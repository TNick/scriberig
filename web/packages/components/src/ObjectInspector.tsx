/** @file Renders a domain-neutral semantic object inspection. */
import type { ReactElement } from "react";
import type { InspectorValue } from "./component-types";

/** Renders identity, revision, properties, and historical state. */
export function ObjectInspector({
  value,
  className,
}: {
  value: InspectorValue;
  className?: string;
}): ReactElement {
  return (
    <section className={className} aria-label="Object inspector">
      <h2>{value.summary.label ?? value.summary.ref.entityId}</h2>
      <p>
        {value.summary.typeId} · {value.summary.ref.revisionId ?? "revision unavailable"}
      </p>
      {value.state && value.state !== "current" ? <p role="status">{value.state}</p> : null}
      {value.description ? <p>{value.description}</p> : null}
      {value.provenance ? <p>Provenance: {value.provenance}</p> : null}
      {value.properties ? (
        <dl>
          {value.properties.map(([key, item]) => (
            <div key={key}>
              <dt>{key}</dt>
              <dd>{item}</dd>
            </div>
          ))}
        </dl>
      ) : null}
      {value.relations ? (
        <ul aria-label="Relations">
          {value.relations.map((relation) => (
            <li key={relation}>{relation}</li>
          ))}
        </ul>
      ) : null}
    </section>
  );
}
