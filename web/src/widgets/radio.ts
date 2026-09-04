// `radio` and `checkboxSet`: the choices laid out at once, under the menu-family floor.
// A `radio` is one value, so the inherited default is one more exclusive choice and no
// separate Unset affordance is needed. A `checkboxSet` is an array field whose checkboxes
// multi-select, so it cannot hold "inherited" as a choice and keeps the Unset button
// beside its Ghost text instead (ADR 0003).
import {
  INHERITED,
  degradeNote,
  ghost,
  inheritedOption,
  shell,
  trim,
  unsetButton,
} from "./common.js";
import type { Ctx } from "./index.js";
import type { FieldNode } from "../types.js";
import { pathText } from "../types.js";

export function mountRadio(node: FieldNode, ctx: Ctx): HTMLElement {
  const box = shell(node);
  const many = node.widget === "checkboxSet";
  const name = pathText(node.path);
  const choices: { value: string; label: string; ghosted: boolean }[] = [];
  if (!many) {
    const inherited = inheritedOption(node);
    choices.push({ value: INHERITED, label: inherited.textContent ?? "", ghosted: true });
  }
  for (const option of node.meta.options) {
    choices.push({ value: JSON.stringify(option.value), label: option.label, ghosted: false });
  }

  for (const choice of choices) {
    const wrap = document.createElement("label");
    wrap.className = choice.ghosted ? "choice ghost" : "choice";
    const input = document.createElement("input");
    input.type = many ? "checkbox" : "radio";
    input.name = name;
    input.value = choice.value;
    input.checked =
      node.presence.kind === "absent"
        ? choice.value === INHERITED
        : node.presence.literal === choice.value;
    input.addEventListener("change", () => {
      if (choice.value === INHERITED) ctx.unset(node.path);
      else ctx.set(node.path, checkedLiteral(box, many, choice.value));
    });
    wrap.append(input, document.createTextNode(choice.label));
    box.append(wrap);
  }
  // A `checkboxSet` has no inherited choice, so absence is the Ghost text plus the button.
  return trim(
    box,
    many ? ghost(node) : null,
    many ? unsetButton(node, ctx) : null,
    degradeNote(node),
  );
}

// One value for a radio; the whole selected set, as a JSON array, for a checkbox set.
function checkedLiteral(box: HTMLElement, many: boolean, just: string): string {
  if (!many) return just;
  const on = [...box.querySelectorAll<HTMLInputElement>("input:checked")].map((i) =>
    JSON.parse(i.value),
  );
  return JSON.stringify(on);
}
