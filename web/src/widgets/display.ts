// `displayOnly`, and every **Locked** or `readOnly` field whatever its Widget: the value is
// shown and no write affordance is offered at all. A disabled input would still be a control
// the user tries to use, so there is none - a `const`, a YAML alias and a `readOnly` key are
// facts about the document, not fields awaiting input (design §7).
import { degradeNote, ghost, literal, shell, trim } from "./common.js";
import type { Ctx } from "./index.js";
import type { FieldNode } from "../types.js";

export function mountDisplay(node: FieldNode, _ctx: Ctx): HTMLElement {
  const box = shell(node);
  box.dataset.readOnly = "true";
  const value = document.createElement("span");
  value.className = "field-literal";
  value.textContent = literal(node);
  return trim(box, node.presence.kind === "absent" ? null : value, ghost(node), degradeNote(node));
}
