// `text`, `textarea`, `masked` and `rawText`: a free-text control. None of the four can
// express "inherit the default" in its own value space - an empty string is a value - so all
// four carry the separate Unset affordance and show the default as Ghost text (ADR 0003).
import { degradeNote, editable, ghost, shell, trim, unsetButton } from "./common.js";
import type { Ctx } from "./index.js";
import type { FieldNode } from "../types.js";

export function mountText(node: FieldNode, ctx: Ctx): HTMLElement {
  const box = shell(node);
  const input =
    node.widget === "textarea"
      ? document.createElement("textarea")
      : document.createElement("input");
  if (input instanceof HTMLInputElement) {
    // `rawText` is the Raw literal fallback: the bytes are shown as authored, never retyped
    // as a value (design §5), so it is still a text input - the field, not the control, is raw.
    input.type = node.widget === "masked" ? "password" : "text";
  }
  input.value = editable(node);
  const d = ghost(node);
  if (d) input.placeholder = d.textContent ?? "";
  // Every widget hands the session a JSON literal. A string field's text is quoted here, so
  // typing `123` into a string stays the string "123"; `rawText` is the exception by
  // definition - it is the Raw literal fallback, and its bytes go through untouched.
  input.addEventListener("change", () =>
    ctx.set(node.path, node.widget === "rawText" ? input.value : JSON.stringify(input.value)),
  );
  return trim(box, input, d, unsetButton(node, ctx), degradeNote(node));
}
