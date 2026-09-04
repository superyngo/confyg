// Partition: how the Form IR is cut into screens (presentation §5.1). The closed set is
// four, of which v0.1 implements two — `tabs` and `wizard` clamp to `sections` until they
// render, so a profile written today stays readable.
import type { FormNode, GroupNode } from "./types.js";

export type PartitionKind = "scroll" | "sections";

export interface Section {
  /** The Group's own key. Empty only for a root that is itself the single section. */
  key: string;
  title: string;
  node: GroupNode;
}

export interface Partition {
  kind: PartitionKind;
  sections: Section[];
  /** Everything at depth 1 that is not a Group: rendered above the sections, in IR order. */
  loose: FormNode[];
}

/** Below this many depth-1 Groups, a two-pane master-detail is a page split for no benefit. */
const SECTIONS_FLOOR = 3;

export function partition(ir: FormNode): Partition {
  const children = ir.kind === "group" ? ir.children : [];
  const sections: Section[] = [];
  const loose: FormNode[] = [];
  for (const child of children) {
    if (child.kind === "group") {
      const key = child.path.length === 0 ? "" : lastKey(child);
      sections.push({ key, title: child.meta.title || key, node: child });
    } else {
      loose.push(child);
    }
  }
  return {
    kind: sections.length >= SECTIONS_FLOOR ? "sections" : "scroll",
    sections,
    loose,
  };
}

function lastKey(node: FormNode): string {
  const seg = node.path[node.path.length - 1];
  return seg && "Key" in seg ? seg.Key : "";
}
