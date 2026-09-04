// Form search, host half. The *matching* is `confyg_form::search`'s and crosses the boundary
// already ranked (presentation §5.3): nothing in this file scores, filters or sorts, because
// two host-side implementations would drift from each other and from the TUI.
//
// What cannot live in the compiler is the reason §5.3 gives for it living there: a result has
// to move the **Partition** to the section containing the hit. That mapping is the Partition's
// own, so it is here.
//
// This is not the Option filter (§3.1). No code and no term is shared with it.
import { button, el, note } from "./dom.js";
import { t } from "./i18n.js";
import type { Partition } from "./partition.js";
import { pathText, type Hit } from "./types.js";

/**
 * The key of the section whose subtree contains `path`, or `null` when the hit sits loose —
 * a loose node is already on the page, so there is nothing to move.
 */
export function sectionFor(path: string, plan: Partition): string | null {
  for (const section of plan.sections) {
    const root = pathText(section.node.path);
    if (path === root || path.startsWith(`${root}.`) || path.startsWith(`${root}[`)) {
      return section.key;
    }
  }
  return null;
}

/**
 * The result list, in the order the compiler returned it. `jump` gets the hit's Path as text,
 * exactly as `data-path` carries it.
 */
export function resultList(hits: Hit[], jump: (path: string) => void): HTMLElement {
  const box = el("div", "search-results");
  if (hits.length === 0) {
    box.append(note(t("form.search.none"), "notice"));
    return box;
  }
  for (const hit of hits) {
    const path = pathText(hit.path);
    const row = el("div", "search-hit");
    row.dataset.path = path;
    const jumper = button("search-jump", hit.title || path);
    // The Path is shown beside the title: two nodes may share a title, and the Path is what
    // makes the row unambiguous.
    const where = el("span", "search-path");
    where.textContent = path;
    jumper.append(where);
    jumper.addEventListener("click", () => jump(path));
    row.append(jumper);
    box.append(row);
  }
  return box;
}
