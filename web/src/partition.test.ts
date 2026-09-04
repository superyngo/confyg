import { describe, expect, test } from "vitest";
import { partition } from "./partition.js";
import type { FormNode, GroupNode } from "./types.js";

function group(key: string, children: FormNode[] = []): GroupNode {
  return {
    kind: "group",
    path: [{ Key: key }],
    meta: { title: key, description: null, violations: [], locked: null, deprecated: false },
    children,
    occupancy: "populated",
    toggle: null,
  };
}

// A root whose depth-1 children are `n` Groups (named a, b, c, …) plus one Field, which is
// never a section: sections are the Schema author's own grouping (presentation §5.1).
function irWithGroups(n: number): FormNode {
  const groups = Array.from({ length: n }, (_, i) => group(String.fromCharCode(97 + i)));
  return {
    kind: "group",
    path: [],
    meta: { title: "root", description: null, violations: [], locked: null, deprecated: false },
    children: [
      ...groups,
      {
        kind: "field",
        path: [{ Key: "loose" }],
        widget: "text",
        intended: "text",
        presence: { kind: "absent", default: null, remarked: null },
        meta: {
          title: "loose",
          description: null,
          violations: [],
          locked: null,
          deprecated: false,
          default: null,
          examples: [],
          required: false,
          readOnly: false,
          writeOnly: false,
          unit: null,
          constraints: [],
          raw: false,
          options: [],
        },
      },
    ],
    occupancy: "populated",
    toggle: null,
  };
}

describe("partition", () => {
  test("fewer than two depth-1 Groups falls back to scroll", () => {
    expect(partition(irWithGroups(1)).kind).toBe("scroll");
    expect(partition(irWithGroups(2)).kind).toBe("sections");
  });

  test("sections come from depth-1 Groups only", () => {
    expect(partition(irWithGroups(4)).sections.map((s) => s.key)).toEqual(["a", "b", "c", "d"]);
  });

  test("a nested Group is not a section of its own", () => {
    const nested = group("a", [group("a-inner")]);
    const ir = irWithGroups(3) as GroupNode;
    ir.children[0] = nested;
    expect(partition(ir).sections.map((s) => s.key)).toEqual(["a", "b", "c"]);
  });

  test("scroll still names its sections, because they are headings", () => {
    expect(partition(irWithGroups(1)).sections.map((s) => s.key)).toEqual(["a"]);
  });
});
