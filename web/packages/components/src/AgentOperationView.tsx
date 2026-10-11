/** @file Renders agent operation state and semantic result links. */
import type { ReactElement } from "react";

/** Keeps semantic result references first-class beside optional output. */
export function AgentOperationView({
  status,
  output,
  links,
  className,
}: {
  status: string;
  output?: string;
  links: readonly string[];
  className?: string;
}): ReactElement {
  return (
    <section className={className} aria-label="Agent operation">
      <p role="status">{status}</p>
      {output ? <pre>{output}</pre> : null}
      <ul aria-label="Semantic results">
        {links.map((link) => (
          <li key={link}>{link}</li>
        ))}
      </ul>
    </section>
  );
}
