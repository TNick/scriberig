/** @file Coordinates exact-snapshot workbook and worksheet summary reads. */
import type { SpreadsheetWorkbookResponse } from "@scriberig/protocol";
import type { ScriberigClient } from "@scriberig/client";

/** Reads ordered worksheet and table summaries from the shared client.
 *
 * Public members: `read`.
 * Private members: the shared client and exact workbook scope.
 */
export class SpreadsheetWorkbookController {
  /** Creates a workbook reader bound to one immutable snapshot.
   * @param client Client exposing the exact-snapshot workbook summary operation.
   * @param project Stable project identity.
   * @param snapshot Exact immutable snapshot identity.
   * @param workbook Stable semantic workbook identity.
   */
  constructor(
    private readonly client: Pick<ScriberigClient, "spreadsheetWorkbook">,
    private readonly project: string,
    private readonly snapshot: string,
    private readonly workbook: string,
  ) {}

  /** Reads ordered worksheet and table metadata at the controller's exact snapshot.
   * @param signal Optional request cancellation signal.
   * @returns Snapshot-bound workbook summary.
   * @throws When the response identity does not match this controller's scope.
   */
  async read(signal?: AbortSignal): Promise<SpreadsheetWorkbookResponse> {
    const response = await this.client.spreadsheetWorkbook(
      {
        project: this.project,
        snapshot: this.snapshot,
        workbook: this.workbook,
      },
      signal,
    );
    if (
      response.project !== this.project ||
      response.snapshot !== this.snapshot ||
      response.workbook !== this.workbook
    ) {
      throw new TypeError("spreadsheet workbook response does not match the requested snapshot");
    }
    return response;
  }
}
