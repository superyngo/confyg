// The violation summary: every attributed Violation, plus whether the document could be
// validated at all (`confyg_form::unknown::summary`). Design §11's C6 item.
//
// The distinction this file exists to keep: an empty list under `available` means nothing is
// wrong, and an empty list under `unavailable` means nobody knows. Rendering both as "no
// problems" is the failure mode D8 was written against — one bad `pattern` costs the whole
// document its validation, not its form.
import { button, el, note } from "./dom.js";
import { t, tArgs } from "./i18n.js";
import { pathText, type Summary } from "./types.js";

/** `jump` gets the hit's Path as text, exactly as `data-path` carries it. */
export function renderSummary(summary: Summary, jump: (path: string) => void): HTMLElement {
  const box = el("section", "summary");

  if (summary.validation.kind === "unavailable") {
    box.dataset.validation = "unavailable";
    box.append(
      note(
        tArgs("form.summary.unavailable", [
          summary.validation.keyword,
          summary.validation.pointer,
        ]),
        "notice",
      ),
    );
    return box;
  }

  box.dataset.validation = "available";
  if (summary.items.length === 0) {
    box.append(note(t("form.summary.none"), "notice"));
    return box;
  }

  const heading = el("h2", "summary-heading");
  heading.textContent = tArgs("form.summary.title", [String(summary.items.length)]);
  box.append(heading);

  const list = el("ul", "summary-items");
  for (const item of summary.items) {
    const path = pathText(item.path);
    const row = el("li", "summary-item");
    // `data-target`, not `data-path`: the summary sits inside the same root as the fields, and
    // a second element claiming `data-path="port"` would shadow the Field it points at — for
    // `reveal`, and for any test or tool that addresses a node by its Path.
    row.dataset.target = path;
    // A button, not a link: the jump moves the Partition in place, and a href would offer
    // the browser a navigation confyg cannot honour.
    const jumper = button("summary-jump", `${item.title || path}: ${item.message}`);
    jumper.addEventListener("click", () => jump(path));
    row.append(jumper);
    list.append(row);
  }
  box.append(list);
  return box;
}
