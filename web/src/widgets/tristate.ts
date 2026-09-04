// `tristate`: a boolean is never a bare checkbox. A checkbox has two states and a boolean
// field has three - inherited, true, false - and collapsing them would write `false` where
// the user meant "leave it to the default" (ADR 0002).
import { INHERITED, degradeNote, inheritedOption, shell, trim } from "./common.js";
import type { Ctx } from "./index.js";
import type { FieldNode } from "../types.js";

export function mountTristate(node: FieldNode, ctx: Ctx): HTMLElement {
  const box = shell(node);
  const select = document.createElement("select");
  select.append(inheritedOption(node));
  for (const value of ["true", "false"]) {
    const o = document.createElement("option");
    o.value = value;
    o.textContent = value;
    select.append(o);
  }
  select.value = node.presence.kind === "absent" ? INHERITED : node.presence.literal;
  select.addEventListener("change", () => {
    if (select.value === INHERITED) ctx.unset(node.path);
    else ctx.set(node.path, select.value);
  });
  return trim(box, select, degradeNote(node));
}
