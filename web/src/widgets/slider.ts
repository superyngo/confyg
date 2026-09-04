// `slider`: only ever resolved when the Schema bounds it on both sides and the host says it
// can slide, so the range's ends are real. The number beside it stays editable, because a
// slider cannot express a precise value and cannot express absence either - hence the
// separate Unset affordance (ADR 0003).
import { degradeNote, ghost, literal, shell, trim, unsetButton } from "./common.js";
import { applyBounds } from "./stepper.js";
import type { Ctx } from "./index.js";
import type { FieldNode } from "../types.js";

export function mountSlider(node: FieldNode, ctx: Ctx): HTMLElement {
  const box = shell(node);
  const range = document.createElement("input");
  range.type = "range";
  applyBounds(range, node.meta.constraints);
  const exact = document.createElement("input");
  exact.type = "number";
  applyBounds(exact, node.meta.constraints);
  for (const input of [range, exact]) input.value = literal(node);

  const sync = (from: HTMLInputElement, to: HTMLInputElement) => {
    from.addEventListener("input", () => {
      to.value = from.value;
    });
    from.addEventListener("change", () => ctx.set(node.path, from.value));
  };
  sync(range, exact);
  sync(exact, range);
  const d = ghost(node);
  if (d) exact.placeholder = d.textContent ?? "";
  return trim(box, range, exact, d, unsetButton(node, ctx), degradeNote(node));
}
