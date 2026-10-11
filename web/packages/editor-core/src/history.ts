/** @file Defines snapshot-aware history coordination. */
export interface HistoryState {
  entityId?: string;
  fromSnapshot?: string;
  toSnapshot?: string;
}
