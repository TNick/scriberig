/** @file Provides migration proposal review. */
import { MuiMigrationPreview } from "@scriberig/components-mui";
import type { ReactElement } from "react";
/** Displays an explicit unsupported state until proposals are exposed publicly. */
/** @returns The rendered MigrationReviewView view or runtime value. */
export function MigrationReviewView(): ReactElement {
  return (
    <section aria-label="Migration review">
      <h1>Migration Review</h1>
      <MuiMigrationPreview
        baseSnapshot="unavailable"
        changes={[]}
        validation="unknown"
        state="conflict"
      />
    </section>
  );
}
