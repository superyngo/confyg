// The parts every widget shares, so the three Presence states look the same in all of them:
// the Ghost text for an Absent value, the Unset affordance for a control that cannot express
// a default in its own value space, and the Degradation notice for a clamped Widget
// (presentation §4). Kept out of `index.ts` so a widget module never imports the registry.
import { t, tArgs } from "../i18n.js";
import type { Ctx } from "./index.js";
import type { FieldNode } from "../types.js";

/** The `inherited` option's value. One string, so no widget invents its own spelling. */
export const INHERITED = "inherited";

export function shell(node: FieldNode): HTMLElement {
  const box = document.createElement("div");
  box.className = "control";
  box.dataset.widget = node.widget;
  return box;
}

/** The default as a person reads it: a string stays a string, anything else is its JSON. */
export function defaultText(node: FieldNode): string | null {
  const d = node.presence.kind === "absent" ? node.presence.default : node.meta.default;
  if (d === null || d === undefined) return null;
  return typeof d === "string" ? d : JSON.stringify(d);
}

/** The literal to seed a control with; empty for an Absent value, which shows Ghost text. */
export function literal(node: FieldNode): string {
  return node.presence.kind === "absent" ? "" : node.presence.literal;
}

/**
 * The same literal as a person edits it: a JSON string loses its quotes, since the quotes
 * belong to the encoding and not to the value. Anything unparseable is shown as authored -
 * an Invalid literal is never rewritten on the way to the screen.
 */
export function editable(node: FieldNode): string {
  const raw = literal(node);
  try {
    const parsed = JSON.parse(raw);
    return typeof parsed === "string" ? parsed : raw;
  } catch {
    return raw;
  }
}

/**
 * Ghost text: what the value *would* be if it stays unwritten. An Absent field with no
 * default says so instead, because "not set" and "set to nothing" are different facts.
 */
export function ghost(node: FieldNode): HTMLElement | null {
  if (node.presence.kind !== "absent") return null;
  const span = document.createElement("span");
  span.className = "ghost";
  const d = defaultText(node);
  span.textContent = d === null ? t("form.presence.unset") : d;
  return span;
}

/** The inherited choice, for a control whose own value space can hold it. */
export function inheritedOption(node: FieldNode): HTMLOptionElement {
  const o = document.createElement("option");
  o.value = INHERITED;
  o.className = "ghost";
  const d = defaultText(node);
  o.textContent = d === null ? t("form.presence.unset") : tArgs("form.presence.inherited", [d]);
  return o;
}

/** The separate Unset affordance, for a control that cannot express absence (ADR 0003). */
export function unsetButton(node: FieldNode, ctx: Ctx): HTMLButtonElement {
  const b = document.createElement("button");
  b.type = "button";
  b.className = "unset";
  b.textContent = t("form.action.unset");
  b.disabled = node.presence.kind === "absent";
  b.addEventListener("click", () => ctx.unset(node.path));
  return b;
}

/**
 * Why this control is not the one the Schema asked for. The clamp already happened in the
 * core; saying so is the host's whole obligation (presentation §4, §6).
 */
export function degradeNote(node: FieldNode): HTMLElement | null {
  if (node.widget === node.intended) return null;
  const p = document.createElement("p");
  p.className = "degrade";
  p.textContent = tArgs("form.degrade", [t(`form.widget.${node.intended}`)]);
  return p;
}

/** Append the parts that are not the control itself, skipping the ones that do not apply. */
export function trim(box: HTMLElement, ...parts: (Node | null)[]): HTMLElement {
  for (const part of parts) if (part) box.append(part);
  return box;
}
