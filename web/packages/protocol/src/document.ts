/** @file Defines exact-snapshot semantic document edit contracts. */

import type { ArtifactEditCapability } from "./artifacts";

/** Requests an exact-snapshot semantic document tree. */
export interface DocumentInspectRequest {
  /** Stable project identity. */
  project: string;
  /** Exact immutable snapshot identity. */
  snapshot: string;
  /** Stable semantic document root identity. */
  document: string;
}

/** Returns ordered semantic sections without package bytes. */
export interface DocumentInspectResponse {
  /** Stable project identity. */
  project: string;
  /** Exact immutable snapshot used by the read. */
  snapshot: string;
  /** Stable semantic document identity. */
  document: string;
  /** Capability of the exact source artifact, when catalogued. */
  artifactCapability?: ArtifactEditCapability | null;
  /** Named document styles used by paragraph and run references. */
  styles: DocumentStyleView[];
  /** Ordered sections in the document. */
  sections: DocumentSectionView[];
}

/** Describes one named, format-owned style definition. */
export interface DocumentStyleView {
  /** Stable style key within the source format. */
  key: string;
  /** Human-readable style name when supplied by the source. */
  name?: string | null;
}

/** Represents one ordered section. */
export interface DocumentSectionView {
  /** Stable section identity. */
  id: string;
  /** Ordered paragraphs and tables. */
  blocks: DocumentBlockView[];
}

/** Represents one typed semantic paragraph or table block. */
export type DocumentBlockView =
  | { kind: "paragraph"; paragraph: DocumentParagraphView }
  | { kind: "table"; table: DocumentTableView };

/** Represents an addressable semantic paragraph. */
export interface DocumentParagraphView {
  /** Stable paragraph identity. */
  id: string;
  /** Ordered authored text runs. */
  runs: DocumentInlineRunView[];
  /** Optional paragraph style reference. */
  style?: DocumentTextStyleRef;
}

/** Represents one inline text run. */
export interface DocumentInlineRunView {
  /** Stable run identity. */
  id: string;
  /** Authored run text. */
  text: string;
  /** Optional run style reference. */
  style?: DocumentTextStyleRef;
  /** Optional resolved hyperlink target. */
  hyperlink?: string;
}

/** Represents one adapter-owned style reference. */
export interface DocumentTextStyleRef {
  /** Adapter-owned style key. */
  key: string;
  /** Adapter that resolves the style. */
  adapter: string;
}

/** Represents one ordered table. */
export interface DocumentTableView {
  /** Stable table identity. */
  id: string;
  /** Authored row order. */
  rows: DocumentRowView[];
}

/** Represents one table row. */
export interface DocumentRowView {
  /** Stable row identity. */
  id: string;
  /** Authored cell order. */
  cells: DocumentCellView[];
}

/** Represents one table cell and its ordered blocks. */
export interface DocumentCellView {
  /** Stable cell identity. */
  id: string;
  /** Ordered paragraphs or nested tables. */
  blocks: DocumentBlockView[];
}

/** Requests a semantic paragraph replacement candidate. */
export interface DocumentParagraphEditRequest {
  /** Stable project identity. */
  project: string;
  /** Exact immutable base snapshot. */
  snapshot: string;
  /** Stable semantic paragraph identity. */
  paragraph: string;
  /** Replacement paragraph text. */
  replacement: string;
}

/** Requests a semantic replacement candidate for one simple table cell. */
export interface DocumentTableCellEditRequest {
  /** Stable project identity. */
  project: string;
  /** Exact immutable base snapshot. */
  snapshot: string;
  /** Stable semantic table-cell identity. */
  cell: string;
  /** Replacement text authored for the cell. */
  replacement: string;
}

/** Returns a reviewable document candidate handle. */
export interface DocumentParagraphEditResponse {
  /** Stable project identity. */
  project: string;
  /** Candidate identity used by shared review operations. */
  candidateId: string;
  /** Exact base snapshot used by candidate creation. */
  baseSnapshot: string;
  /** Project-relative DOCX path. */
  path: string;
  /** Candidate validation state. */
  validation:
    { status: "pending" } | { status: "passed" } | { status: "failed"; diagnostics: string[] };
}

/** Requests a semantic paragraph append candidate for one exact section. */
export interface DocumentParagraphInsertRequest {
  /** Stable project identity. */
  project: string;
  /** Exact immutable base snapshot. */
  snapshot: string;
  /** Stable semantic section identity receiving the paragraph. */
  section: string;
  /** Authored text for the appended paragraph. */
  text: string;
}

/** Returns a reviewable inserted-paragraph candidate handle. */
export interface DocumentParagraphInsertResponse {
  /** Stable project identity. */
  project: string;
  /** Candidate identity used by shared validation and review operations. */
  candidateId: string;
  /** Exact base snapshot used by candidate creation. */
  baseSnapshot: string;
  /** Project-relative DOCX path. */
  path: string;
  /** Candidate validation state. */
  validation:
    { status: "pending" } | { status: "passed" } | { status: "failed"; diagnostics: string[] };
}
