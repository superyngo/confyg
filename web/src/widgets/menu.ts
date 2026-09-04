// `menu`: a select over the choices the core labelled. The inherited default is one more
// option, so no separate Unset affordance is needed (ADR 0003). `filterableMenu` maps here
// too - the Host clamp turns it into `menu` before a host ever sees it, and the Option filter
// itself is v0.2.
import { INHERITED, degradeNote, literal, shell, inheritedOption, trim } from "./common.js";
import type { Ctx } from "./index.js";
import type { FieldNode } from "../types.js";

export function mountMenu(node: FieldNode, ctx: Ctx): HTMLElement {
  const box = shell(node);
  const select = document.createElement("select");
  select.append(inheritedOption(node));
  for (const option of node.meta.options) {
    const o = document.createElement("option");
    o.value = JSON.stringify(option.value);
    o.textContent = option.label;
    select.append(o);
  }
  select.value = node.presence.kind === "absent" ? INHERITED : literal(node);
  select.addEventListener("change", () => {
    if (select.value === INHERITED) ctx.unset(node.path);
    else ctx.set(node.path, select.value);
  });
  return trim(box, select, degradeNote(node));
}
