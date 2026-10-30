/** @file Coordinates bounded pages of rich artifact inspection summaries. */
import type { ArtifactInspectResponse } from "@scriberig/protocol";
import type { ScriberigClient } from "@scriberig/client";

/** Maximum artifact summaries requested in one application page. */
const MAX_ARTIFACT_PAGE_SIZE = 100;

/** Controls bounded artifact inspection over the shared semantic client.
 *
 * Public members: `read`, `next`, `previous`, `canPrevious`, and `currentPage`.
 * Private members: client, project, page size, current page, and current offset.
 */
export class ArtifactPageController {
  private page?: ArtifactInspectResponse;
  private offset = 0;

  /** Creates a controller bound to one project artifact catalog.
   * @param client Client exposing bounded artifact inspection.
   * @param project Stable project identity.
   * @param pageSize Maximum summaries per request, from one through one hundred.
   * @throws When the page size is outside the shared API limit.
   */
  constructor(
    private readonly client: Pick<ScriberigClient, "inspectArtifacts">,
    private readonly project: string,
    private readonly pageSize = 20,
  ) {
    if (!Number.isInteger(pageSize) || pageSize < 1 || pageSize > MAX_ARTIFACT_PAGE_SIZE) {
      throw new RangeError("artifact page size must be between one and one hundred");
    }
  }

  /** Reads one bounded artifact page.
   * @param offset Zero-based artifact offset.
   * @param signal Optional request cancellation signal.
   * @returns Normalized artifact summaries and next offset.
   * @throws When the offset is invalid or the client request fails.
   */
  async read(offset = 0, signal?: AbortSignal): Promise<ArtifactInspectResponse> {
    if (!Number.isInteger(offset) || offset < 0) {
      throw new RangeError("artifact page offset must be a non-negative integer");
    }
    const page = await this.client.inspectArtifacts(
      { project: this.project, offset, limit: this.pageSize },
      signal,
    );
    this.offset = offset;
    this.page = page;
    return page;
  }

  /** Reads the next page when the current result provides a continuation.
   * @param signal Optional request cancellation signal.
   * @returns Next page, or undefined when no continuation exists.
   */
  async next(signal?: AbortSignal): Promise<ArtifactInspectResponse | undefined> {
    if (this.page?.nextOffset === undefined) return undefined;
    return this.read(this.page.nextOffset, signal);
  }

  /** Reads the previous page when the current offset is nonzero.
   * @param signal Optional request cancellation signal.
   * @returns Previous page, or undefined when already at the first page.
   */
  async previous(signal?: AbortSignal): Promise<ArtifactInspectResponse | undefined> {
    if (this.page === undefined || this.offset === 0) return undefined;
    return this.read(Math.max(0, this.offset - this.pageSize), signal);
  }

  /** Reports whether the loaded page has an earlier page.
   * @returns True when a page is loaded after offset zero.
   */
  canPrevious(): boolean {
    return this.page !== undefined && this.offset > 0;
  }

  /** Returns the latest page completed by this controller.
   * @returns Current artifact page, if one has loaded.
   */
  currentPage(): ArtifactInspectResponse | undefined {
    return this.page;
  }
}
