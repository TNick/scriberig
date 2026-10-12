/** @file Defines shared, styling-neutral component contracts. */

import type { ReactNode } from "react";
import type { KnowledgeTree, TreeNode } from "@scriberig/editor-core";
import type { ObjectSummary, SemanticDiff } from "@scriberig/protocol";

/** Common styling and DOM extension properties. */
export interface ComponentProps {
  /** Optional caller-owned class name. */
  className?: string;
  /** Optional caller-owned children. */
  children?: ReactNode;
}

/** Props for a lazy semantic object tree. */
export interface ObjectTreeProps extends ComponentProps {
  /** Headless tree controller. */
  tree: KnowledgeTree;
  /** Root node handle. */
  rootId: string;
  /** Called when a row is selected. */
  onSelect?: (node: TreeNode) => void;
  /** Loads the next page for a node. */
  onLoadMore?: (node: TreeNode) => void;
}

/** One semantic object inspection value. */
export interface InspectorValue {
  /** Object summary. */
  summary: ObjectSummary;
  /** Optional description. */
  description?: string;
  /** Optional provenance label. */
  provenance?: string;
  /** Whether this is historical or stale. */
  state?: "current" | "historical" | "stale";
  /** Summary properties. */
  properties?: readonly [string, string][];
  /** Available relations. */
  relations?: readonly string[];
}

/** One semantic diff entry. */
export interface DiffEntry {
  /** Stable entry identity. */
  id: string;
  /** Change kind. */
  kind: string;
  /** Human-readable summary. */
  summary: string;
}

/** One semantic migration change or legacy display-only change. */
export type MigrationChange = SemanticDiff | string;

/** One session branch summary. */
export interface BranchSummary {
  /** Stable branch identity. */
  id: string;
  /** Branch title. */
  title: string;
  /** Optional status. */
  status?: string;
  /** Nested branch summaries. */
  children?: readonly BranchSummary[];
}

/** One registered projection handler. */
export interface ProjectionHandler {
  /** Projection identity handled by this renderer. */
  id: string;
  /** Renders the selected projection. */
  render: (context: { selection?: string }) => ReactNode;
}
