// `stepper`: a number input with the Schema's bounds as guidance, not as a gate - a value
// outside them is written and warned about, never refused (design §5). A number input cannot
// hold "inherited", so the Unset affordance stays separate (ADR 0003).
import { degradeNote, ghost, literal, shell, trim, unsetButton } from "./common.js";
import type { Ctx } from "./index.js";
import type { Constraint, FieldNode } from "../types.js";

export function mountStepper(node: FieldNode, ctx: Ctx): HTMLElement {
  const box = shell(node);
  const input = document.createElement("input");
  input.type = "number";
  applyBounds(input, node.meta.constraints);
  input.value = literal(node);
  const d = ghost(node);
  if (d) input.placeholder = d.textContent ?? "";
  input.addEventListener("change", () => ctx.set(node.path, input.value));
  return trim(box, input, d, unsetButton(node, ctx), degradeNote(node));
}

export function applyBounds(input: HTMLInputElement, constraints: Constraint[]): void {
  for (const c of constraints) {
    if (c.kind === "minimum") input.min = String(c.value);
    if (c.kind === "maximum") input.max = String(c.value);
    if (c.kind === "multipleOf") input.step = String(c.value);
  }
}
