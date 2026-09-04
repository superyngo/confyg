// Repeat groups: a card per object entry, a row per scalar one, a count badge, and the two
// bounds gates. Design §3's Repeat node, presentation §5's card layout.
//
// The gates mirror `confyg-session/src/lower.rs`, which refuses `AddRepeatItem` at `maxItems`
// and `RemoveRepeatItem` at `minItems`. The host has the same `bounds` in hand, so offering a
// button the core would refuse is a bug the renderer can see coming — the predicates here are
// deliberately the same two comparisons.
import { button, chrome, el, groupHeading, label } from "./dom.js";
import { t, tArgs } from "./i18n.js";
import { pathText, type Bounds, type FormNode, type Path, type RepeatNode } from "./types.js";
import { editable } from "./widgets/common.js";

/** What a Repeat asks the session for. Widgets never see these: only a container adds. */
export interface RepeatCtx {
  addItem(path: Path): void;
  removeItem(path: Path, index: number): void;
  /** The shell's own walk — a card's contents are ordinary nodes. */
  renderChild(node: FormNode, depth: number): HTMLElement;
}

/** `(3/5)` when the Schema set a ceiling, `(3)` when it did not. */
export function countText(bounds: Bounds, count: number): string {
  return bounds.max === null ? `(${count})` : `(${count}/${bounds.max})`;
}

/**
 * The card's title: the `labelFrom` entry's own value when it has one, else the item's title
 * (or the collection's) with its ordinal.
 *
 * `labelFrom` is the core's choice, not the host's — a host that re-derived it would label the
 * same array differently in the Web and the TUI.
 */
export function cardTitle(node: RepeatNode, index: number): string {
  const item = node.items[index];
  const labelled = node.labelFrom === null ? null : labelValue(item, node.labelFrom);
  if (labelled !== null) return labelled;
  const own = item && "meta" in item ? item.meta.title : "";
  const title = own || node.meta.title || pathText(node.path);
  return tArgs("form.repeat.ordinal", [title, String(index + 1)]);
}

export function renderRepeat(node: RepeatNode, depth: number, ctx: RepeatCtx): HTMLElement {
  const box = el("section", "node repeat");
  box.dataset.path = pathText(node.path);
  box.dataset.occupancy = node.occupancy;

  const heading = groupHeading(node, depth);
  const count = el("span", "repeat-count");
  count.textContent = countText(node.bounds, node.items.length);
  heading.append(count);
  box.append(heading, ...chrome(node.meta));

  const items = el("div", "repeat-items");
  const atMin = node.bounds.min !== null && node.items.length <= node.bounds.min;
  node.items.forEach((item, index) => {
    items.append(entry(node, item, index, depth, atMin, ctx));
  });
  box.append(items);

  const add = button("repeat-add", t("form.repeat.add"));
  const atMax = node.bounds.max !== null && node.items.length >= node.bounds.max;
  add.disabled = atMax;
  if (atMax && node.bounds.max !== null) {
    add.title = tArgs("form.repeat.atMax", [String(node.bounds.max)]);
  }
  add.addEventListener("click", () => ctx.addItem(node.path));
  box.append(add);
  return box;
}

/**
 * One entry. An object entry is a card with its own heading; a scalar entry is a row, because
 * a card around a single text field is chrome with nothing in it.
 */
function entry(
  node: RepeatNode,
  item: FormNode,
  index: number,
  depth: number,
  atMin: boolean,
  ctx: RepeatCtx,
): HTMLElement {
  const scalar = item.kind !== "group";
  const wrap = el("div", scalar ? "repeat-row" : "card");
  // The *entry* index, which is what `RemoveRepeatItem` takes: `lower.rs` indexes the IR's
  // items, never the Document's children, so a comment is never counted here.
  wrap.dataset.index = String(index);

  const head = el("div", scalar ? "row-head" : "card-head");
  head.append(label(cardTitle(node, index)));
  const remove = button("repeat-remove", t("form.repeat.remove"));
  remove.disabled = atMin;
  if (atMin && node.bounds.min !== null) {
    remove.title = tArgs("form.repeat.atMin", [String(node.bounds.min)]);
  }
  remove.addEventListener("click", () => ctx.removeItem(node.path, index));
  head.append(remove);

  wrap.append(head, ctx.renderChild(item, depth + 1));
  return wrap;
}

/** The `labelFrom` child's literal, rendered for a human. Absent or unset gives `null`. */
function labelValue(item: FormNode | undefined, key: string): string | null {
  if (!item || item.kind !== "group") return null;
  for (const child of item.children) {
    if (child.kind !== "field") continue;
    const seg = child.path[child.path.length - 1];
    if (!seg || !("Key" in seg) || seg.Key !== key) continue;
    // Task 18's own unquoting, not a second copy of it: a JSON string loses the quotes that
    // belong to the encoding, an unparseable literal is shown as authored, and an Absent
    // value gives the empty string — which is no label, so the ordinal takes over.
    return editable(child) || null;
  }
  return null;
}
