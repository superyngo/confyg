// Partition: how the Form IR is cut into screens (presentation §5.1). The closed set is
// four, of which v0.1 implements two — `tabs` and `wizard` clamp to `sections` until they
// render, so a profile written today stays readable.
import type { FormNode, GroupNode, RepeatNode } from "./types.js";

export type PartitionKind = "scroll" | "sections";

export interface Section {
  /** The Group's own key. Empty only for a root that is itself the single section. */
  key: string;
  title: string;
  node: GroupNode;
  /**
   * The Group is one Repeat and nothing else — the `servers` shape. The layout gives it an
   * entry column instead of a stack of cards, so choosing an entry and editing its fields
   * are two different places rather than one long page.
   */
  repeat: RepeatNode | null;
}

export interface Partition {
  kind: PartitionKind;
  sections: Section[];
  /** Everything at depth 1 that is not a Group: rendered above the sections, in IR order. */
  loose: FormNode[];
}

/**
 * Below this many depth-1 Groups, a two-pane master-detail is a page split for no benefit.
 * Two, not three: with two sections the split already beats a scroll, because the section
 * list is also where the violation counts live.
 */
const SECTIONS_FLOOR = 2;

export function partition(ir: FormNode): Partition {
  const children = ir.kind === "group" ? ir.children : [];
  const sections: Section[] = [];
  const loose: FormNode[] = [];
  for (const child of children) {
    if (child.kind === "group") {
      const key = child.path.length === 0 ? "" : lastKey(child);
      sections.push({ key, title: child.meta.title || key, node: child, repeat: soleRepeat(child) });
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

/** The Group's only child, when that child is a Repeat. */
function soleRepeat(node: GroupNode): RepeatNode | null {
  const kids = node.children;
  return kids.length === 1 && kids[0].kind === "repeat" ? kids[0] : null;
}

function lastKey(node: FormNode): string {
  const seg = node.path[node.path.length - 1];
  return seg && "Key" in seg ? seg.Key : "";
}
