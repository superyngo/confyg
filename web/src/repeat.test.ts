// Repeat cards over the snapshot the real core produced: `servers` sits at its `maxItems`
// (3 of 3, object items) and `tags` at its `minItems` (2 of 4, scalar items), so both bounds
// gates and both item shapes come from the compiler rather than from a hand-built IR.
//
//   printf 'json\nq\n' | cargo run -q -p confyg-session --example try -- \
//     crates/confyg-session/examples/demo.schema.json crates/confyg-session/examples/demo.toml
import { describe, expect, test } from "vitest";
import { cardTitle, countText } from "./repeat.js";
import { render, type HostCtx } from "./render.js";
import snapshot from "./__fixtures__/demo-snapshot.json";
import { pathText, type FormNode, type Path, type RepeatNode, type SetterSnapshot } from "./types.js";

const demo = snapshot as unknown as SetterSnapshot;

interface Call {
  kind: string;
  path: string;
  index?: number;
}

function rendered(calls: Call[] = []): HTMLElement {
  const ctx: HostCtx = {
    set: () => {},
    unset: () => {},
    addItem: (path) => calls.push({ kind: "addItem", path: pathText(path) }),
    removeItem: (path, index) => calls.push({ kind: "removeItem", path: pathText(path), index }),
    toggleGroup: () => {},
  };
  const root = document.createElement("main");
  render(demo, root, ctx);
  return root;
}

function repeatEl(path: string): HTMLElement {
  const el = rendered().querySelector<HTMLElement>(`.node.repeat[data-path="${path}"]`);
  if (!el) throw new Error(`no Repeat rendered at ${path}`);
  return el;
}

function nodeAt(path: string, from: FormNode = demo.ir): RepeatNode {
  if (pathText(from.path as Path) === path && from.kind === "repeat") return from;
  const kids = from.kind === "group" ? from.children : from.kind === "repeat" ? from.items : [];
  for (const kid of kids) {
    try {
      return nodeAt(path, kid);
    } catch {
      // not in this subtree
    }
  }
  throw new Error(`no Repeat node at ${path}`);
}

function addButton(el: HTMLElement): HTMLButtonElement {
  const button = el.querySelector<HTMLButtonElement>(".repeat-add");
  if (!button) throw new Error("no Add button");
  return button;
}

describe("repeat", () => {
  test("a Repeat group shows a count badge and gates + at maxItems", () => {
    const servers = repeatEl("servers");
    expect(servers.querySelector(".repeat-count")?.textContent).toBe("(3/3)");
    expect(addButton(servers).disabled).toBe(true);

    const tags = repeatEl("tags");
    expect(tags.querySelector(".repeat-count")?.textContent).toBe("(2/4)");
    expect(addButton(tags).disabled).toBe(false);
  });

  test("an unbounded Repeat counts without a ceiling it does not have", () => {
    expect(countText({ min: null, max: null }, 3)).toBe("(3)");
    expect(countText({ min: 1, max: 5 }, 3)).toBe("(3/5)");
  });

  test("Remove is gated at minItems, mirroring the core's own refusal", () => {
    // `lower.rs` refuses both edges; a host that offered them anyway would be asking for a
    // Refused it had the data to predict.
    const tagRemoves = repeatEl("tags").querySelectorAll<HTMLButtonElement>(".repeat-remove");
    expect(tagRemoves).toHaveLength(2);
    expect([...tagRemoves].every((b) => b.disabled)).toBe(true);

    const serverRemoves = repeatEl("servers").querySelectorAll<HTMLButtonElement>(".repeat-remove");
    expect([...serverRemoves].every((b) => b.disabled)).toBe(false);
  });

  test("a card is titled from labelFrom, falling back to an ordinal", () => {
    const servers = nodeAt("servers");
    expect(servers.labelFrom).toBe("host");
    expect(cardTitle(servers, 0)).toBe("a.example");
    // `"a.example"` arrives as a JSON literal, quotes and all; the card shows the value, not
    // its encoding. This is `widgets/common.ts` `editable`, the same unquoting every widget
    // uses — a second copy of it here would drift from the field a card contains.

    // The same node with that one label value taken away: the item's own `items.title`
    // carries the fallback, never the raw index alone.
    const blank = structuredClone(servers);
    const item = blank.items[1];
    if (item.kind !== "group") throw new Error("expected object items");
    const labelled = item.children[0];
    if (labelled.kind !== "field") throw new Error("expected a Field to label from");
    labelled.presence = { kind: "absent", default: null, remarked: null };
    expect(cardTitle(blank, 1)).toBe("Server #2");

    // An Invalid literal is never rewritten on the way to the screen: the card shows the
    // bytes as authored, so a card title cannot disagree with the field inside it.
    const bad = structuredClone(servers);
    const first = bad.items[0];
    if (first.kind !== "group") throw new Error("expected object items");
    const host = first.children[0];
    if (host.kind !== "field") throw new Error("expected a Field to label from");
    host.presence = { kind: "invalid", literal: "not json", violations: [] };
    expect(cardTitle(bad, 0)).toBe("not json");
  });

  test("a scalar Repeat renders rows, not cards", () => {
    const tags = repeatEl("tags");
    expect(tags.querySelectorAll(".card")).toHaveLength(0);
    expect(tags.querySelectorAll(".repeat-row")).toHaveLength(2);
    // With no labelFrom and no per-item title, the Repeat's own title carries the ordinal.
    expect(cardTitle(nodeAt("tags"), 0)).toBe("Tags #1");
  });

  test("the + and − buttons ask the core, never mutate the DOM", () => {
    const calls: Call[] = [];
    const root = rendered(calls);
    root.querySelector<HTMLButtonElement>('.node.repeat[data-path="tags"] .repeat-add')?.click();
    const removes = root.querySelectorAll<HTMLButtonElement>(
      '.node.repeat[data-path="servers"] .repeat-remove',
    );
    removes[2].click();
    expect(calls).toEqual([
      { kind: "addItem", path: "tags" },
      { kind: "removeItem", path: "servers", index: 2 },
    ]);
  });

  test("a card carries its entry index, which is the index the core removes by", () => {
    // The IR counts entries; `Target.index` counts children, comments included. A card that
    // published a child index would delete a comment (`crates.md` Index spaces).
    const cards = repeatEl("servers").querySelectorAll<HTMLElement>(".card");
    expect([...cards].map((c) => c.dataset.index)).toEqual(["0", "1", "2"]);
  });
});
