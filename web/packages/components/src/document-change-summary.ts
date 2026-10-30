/** @file Formats bounded semantic paragraph changes for shared candidate review. */

const MAX_PARAGRAPH_PREVIEW = 120;

/** Summarizes one exact paragraph replacement without including unbounded text.
 * @param paragraph Stable semantic paragraph identity.
 * @param before Authored paragraph text at the candidate base snapshot.
 * @param after Proposed replacement paragraph text.
 * @returns A bounded human readable old-to-new semantic change summary.
 */
export function documentParagraphChangeSummary(
  paragraph: string,
  before: string,
  after: string,
): string {
  return `Paragraph ${paragraph} changes from “${preview(before)}” to “${preview(after)}”.`;
}

/** Summarizes one exact table-cell replacement without including unbounded text.
 * @param cell Stable semantic table-cell identity.
 * @param before Authored cell text at the candidate base snapshot.
 * @param after Proposed replacement cell text.
 * @returns A bounded human readable old-to-new semantic change summary.
 */
export function documentTableCellChangeSummary(
  cell: string,
  before: string,
  after: string,
): string {
  return `Table cell ${cell} changes from “${preview(before)}” to “${preview(after)}”.`;
}

/** Truncates paragraph text by Unicode code point for bounded review display.
 * @param text Authored paragraph text.
 * @returns The exact text when short, or a preview with an omission marker.
 */
function preview(text: string): string {
  const characters = Array.from(text);
  return characters.length > MAX_PARAGRAPH_PREVIEW
    ? `${characters.slice(0, MAX_PARAGRAPH_PREVIEW).join("")}…`
    : text;
}
