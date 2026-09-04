// The per-field affordances that are not the control: the hover tooltip and the "..." menu.
//
// ADR 0003 gave every widget its own Unset button. That put a permanent, mostly-idle
// button on every row; the affordance is real but it is not worth a track of its own. It
// moves here as the menu's first item, so a row is label | control | one 30px button, and
// the menu is also where the read-only facts live (raw literal, constraints, schema info)
// that previously had nowhere to go.
import { t } from "./i18n.js";
import { el } from "./dom.js";
import { pathText, type FieldNode } from "./types.js";
import type { Ctx } from "./widgets/index.js";

/** One open menu at a time, so a second "..." click replaces rather than stacks. */
let open: HTMLElement | null = null;

function close(): void {
  open?.remove();
  open = null;
}

document.addEventListener("keydown", (e) => {
  if (e.key === "Escape") close();
});

/**
 * What the Schema says about this field, as a person reads it. The core does not hand the
 * host the compiled subschema (see PATCH.md), so this is assembled from `NodeMeta` and the
 * resolved Widget: type-ish facts first, then the default, then the flags.
 */
export function schemaText(node: FieldNode): string {
  const parts: string[] = [pathText(node.path)];
  parts.push(t("form.widget." + node.widget));
  if (node.widget !== node.intended) {
    parts.push(t("form.degrade.short") + " " + t("form.widget." + node.intended));
  }
  const meta = node.meta;
  if (meta.unit) parts.push(meta.unit);
  if (meta.default !== null && meta.default !== undefined) {
    const d = typeof meta.default === "string" ? meta.default : JSON.stringify(meta.default);
    parts.push(t("form.presence.default") + " " + d);
  }
  if (meta.required) parts.push(t("form.badge.required"));
  if (meta.locked) parts.push(t("form.badge.locked"));
  if (meta.deprecated) parts.push(t("form.badge.deprecated"));
  return parts.join(" \u00b7 ");
}

/** The raw literal exactly as the file holds it; an Absent value has none to show. */
function rawText(node: FieldNode): string {
  return node.presence.kind === "absent" ? t("form.presence.unset") : node.presence.literal;
}

async function copy(text: string): Promise<void> {
  try {
    await navigator.clipboard.writeText(text);
  } catch {
    /* A denied clipboard is not an error worth interrupting an edit for. */
  }
}

function item(menu: HTMLElement, key: string, act: () => void, enabled = true): void {
  const b = el("button", "row-menu-item") as HTMLButtonElement;
  b.type = "button";
  b.textContent = t(key);
  b.disabled = !enabled;
  b.addEventListener("click", () => {
    close();
    act();
  });
  menu.append(b);
}

export function toast(text: string): void {
  const box = document.getElementById("toast");
  if (!box) return;
  box.textContent = text;
  box.hidden = false;
  window.setTimeout(() => {
    box.hidden = true;
  }, 2200);
}

/**
 * Build and place the menu. On a pointer it is anchored under the button; in the pushed
 * shell the stylesheet turns the same element into a bottom sheet, so there is one menu
 * definition and not two.
 */
export function showRowMenu(node: FieldNode, ctx: Ctx, anchor: HTMLElement): void {
  close();
  const menu = el("div", "row-menu");
  menu.setAttribute("role", "menu");
  const head = el("p", "row-menu-path");
  head.textContent = pathText(node.path);
  menu.append(head);

  item(menu, "form.action.unset", () => ctx.unset(node.path), node.presence.kind !== "absent");
  item(menu, "form.action.copyValue", () => {
    void copy(rawText(node));
    toast(t("form.toast.copied"));
  });
  item(menu, "form.action.rawLiteral", () => toast(t("form.action.rawLiteral") + ": " + rawText(node)));
  item(menu, "form.action.schemaInfo", () => toast(schemaText(node)));

  const scrim = el("button", "scrim") as HTMLButtonElement;
  scrim.type = "button";
  scrim.tabIndex = -1;
  scrim.addEventListener("click", close);

  const host = document.body;
  host.append(scrim, menu);
  const box = anchor.getBoundingClientRect();
  menu.style.left = Math.min(box.left - 180, window.innerWidth - 230) + "px";
  menu.style.top = box.bottom + 6 + "px";
  open = menu;
  // The scrim belongs to this menu; closing takes both.
  menu.addEventListener("remove", () => scrim.remove());
  const observer = new MutationObserver(() => {
    if (!menu.isConnected) {
      scrim.remove();
      observer.disconnect();
    }
  });
  observer.observe(host, { childList: true });
}

/** The 30px track at the end of every field row. */
export function rowMenuButton(node: FieldNode, ctx: Ctx): HTMLButtonElement {
  const b = el("button", "row-menu-button") as HTMLButtonElement;
  b.type = "button";
  b.textContent = "\u22ef";
  b.title = t("form.action.more");
  b.setAttribute("aria-haspopup", "menu");
  b.addEventListener("click", () => showRowMenu(node, ctx, b));
  return b;
}

/**
 * The same menu from a long press, for the pushed shell where the row has no button.
 * 500ms, cancelled by a move — a scroll must never open a menu.
 */
export function attachLongPress(row: HTMLElement, node: FieldNode, ctx: Ctx): void {
  let timer = 0;
  const cancel = (): void => window.clearTimeout(timer);
  row.addEventListener("pointerdown", (e) => {
    if (e.pointerType === "mouse") return;
    timer = window.setTimeout(() => showRowMenu(node, ctx, row), 500);
  });
  for (const ev of ["pointerup", "pointercancel", "pointermove"]) row.addEventListener(ev, cancel);
  row.addEventListener("contextmenu", (e) => {
    e.preventDefault();
    showRowMenu(node, ctx, row);
  });
}

/**
 * One gate for every integer control: non-digits can never reach the CST, and the Schema's
 * ceiling clamps as you type. A below-minimum value stays reachable on purpose — rewriting
 * a half-typed number under the caret is worse than showing the violation it already is.
 */
export function guardInt(input: HTMLInputElement, max?: number | null): void {
  input.type = "number";
  input.inputMode = "numeric";
  input.step = "1";
  if (max !== null && max !== undefined) input.max = String(max);
  input.addEventListener("input", () => {
    const digits = input.value.replace(/[^\d]/g, "").replace(/^0+(?=\d)/, "");
    if (digits === "") {
      input.value = "";
      return;
    }
    let n = Number.parseInt(digits, 10);
    if (max !== null && max !== undefined && n > max) n = max;
    const next = String(n);
    if (input.value !== next) input.value = next;
  });
}
