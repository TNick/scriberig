/** @file Defines headless semantic diff state. */
export interface DiffState {
  from?: string;
  to?: string;
  loading: boolean;
  error?: string;
}
