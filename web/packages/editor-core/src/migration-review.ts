/** @file Defines local migration review state. */
export interface MigrationReviewState {
  proposalId: string;
  baseSnapshot?: string;
  candidateSnapshot?: string;
  selectedChange?: string;
  stale: boolean;
  conflict: boolean;
}
