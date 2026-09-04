// The Presence-independent chrome: element builders, the description/violation block, and a
// container's heading. Extracted from `render.ts` when `repeat.ts`, `summary.ts` and
// `search.ts` all needed the same builders — the alternatives were a cycle
// (`render.ts` -> `repeat.ts` -> `render.ts`) or four copies of `el`.
//
// Nothing here decides anything about a form: every one of these takes text or a `NodeMeta`
// the core already filled in.
import { pathText, type FormNode, type GroupNode, type NodeMeta, type RepeatNode, type Violation } from "./types.js";

export function el(tag: string, cls: string): HTMLElement {
  const node = document.createElement(tag);
  node.className = cls;
  return node;
}

export function label(text: string): HTMLElement {
  const node = el("span", "field-label");
  node.textContent = text;
  return node;
}

export function note(text: string, cls: string): HTMLElement {
  const node = el("p", cls);
  node.textContent = text;
  return node;
}

export function badge(text: string, cls: string): HTMLElement {
  const node = el("span", `badge ${cls}`);
  node.textContent = text;
  return node;
}

export function button(cls: string, text: string): HTMLButtonElement {
  const node = el("button", cls) as HTMLButtonElement;
  node.type = "button";
  node.textContent = text;
  return node;
}

/** Description and violations: the same chrome for every node kind. */
export function chrome(meta: NodeMeta): HTMLElement[] {
  const parts: HTMLElement[] = [];
  if (meta.description) parts.push(note(meta.description, "description"));
  parts.push(...meta.violations.map((v) => note(v.message, "violation")));
  return parts;
}

export function violationsIn(node: FormNode): Violation[] {
  const own = "meta" in node ? node.meta.violations : [];
  const kids = node.kind === "group" ? node.children : node.kind === "repeat" ? node.items : [];
  return kids.reduce<Violation[]>((all, kid) => all.concat(violationsIn(kid)), [...own]);
}

export function groupHeading(node: GroupNode | RepeatNode, depth: number): HTMLElement {
  const h = el(`h${Math.min(depth + 1, 6)}`, "node-heading");
  h.textContent = node.meta.title || pathText(node.path);
  return h;
}
