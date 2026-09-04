// The renderer shell: a recursive Form IR walk dispatching on `kind`, laid out by the
// Partition (presentation §5). New code, not a port — confy's `render.ts` and its `ViewRow`
// serve a tree editor whose row model is a value/type pair, and ADR 0002 keeps the two
// apart.
//
// Field controls come from the widget registry (`widgets/index.ts`) and Repeat cards from
// `repeat.ts`; this file owns everything around them: sections, headings, group containers,
// the violation summary, and the Presence-independent chrome.
import { badge, chrome, el, groupHeading, label, note, violationsIn } from "./dom.js";
import { t } from "./i18n.js";
import { partition, type Partition, type Section } from "./partition.js";
import { renderRepeat as renderRepeatCards } from "./repeat.js";
import { sectionFor } from "./search.js";
import { renderSummary } from "./summary.js";
import {
  pathText,
  type FieldNode,
  type FormNode,
  type GroupNode,
  type Path,
  type RepeatNode,
  type SetterSnapshot,
} from "./types.js";
import { mount, type Ctx } from "./widgets/index.js";

/**
 * What the shell asks the session for. A widget only ever sets or unsets a value (`Ctx`);
 * the container intents live here because only a container can offer them.
 */
export interface HostCtx extends Ctx {
  addItem(path: Path): void;
  removeItem(path: Path, index: number): void;
  toggleGroup(path: Path, enable: boolean): void;
}

// The host renders one document at a time, so the session the controls write to is ambient
// rather than threaded through every layout function.
let active: HostCtx = {
  set: () => {},
  unset: () => {},
  addItem: () => {},
  removeItem: () => {},
  toggleGroup: () => {},
};

// The rendered document, kept so a summary item or a search hit can jump to a node that may
// be in a section the user is not looking at.
let current: { root: HTMLElement; plan: Partition; show: ((key: string) => void) | null } | null =
  null;

export function render(snapshot: SetterSnapshot, root: HTMLElement, ctx?: HostCtx): void {
  if (ctx) active = ctx;
  const plan = partition(snapshot.ir);
  current = { root, plan, show: null };
  const body = plan.kind === "sections" ? sectionsLayout(plan) : scrollLayout(plan);
  // The summary sits above the form in both Partitions: a violation the user cannot see is a
  // violation they cannot fix, whichever screen holds the field.
  root.replaceChildren(renderSummary(snapshot.summary, reveal), body);
  root.dataset.partition = plan.kind;
}

/**
 * Move to the node at `path` and mark it: the section containing it first, then the node.
 *
 * Free navigation, so this only ever *shows* — nothing is withheld and nothing is disabled
 * on the way (ADR 0004 decision 5).
 */
export function reveal(path: string): void {
  if (!current) return;
  const key = sectionFor(path, current.plan);
  if (key !== null && current.show) current.show(key);
  // Compared rather than selected: a Schema key may hold characters an attribute selector
  // would have to escape.
  const target = [...current.root.querySelectorAll<HTMLElement>("[data-path]")].find(
    (node) => node.dataset.path === path,
  );
  if (!target) return;
  for (const prior of current.root.querySelectorAll(".revealed")) {
    prior.classList.remove("revealed");
  }
  // A class, not `focus()`: a section is not a focusable thing, and pulling focus somewhere
  // the user did not ask for fights a screen reader.
  target.classList.add("revealed");
  target.scrollIntoView?.({ block: "nearest" });
}

// `scroll`: one page, and a section is its Group's own heading. A section is rendered
// through `renderGroup` like any other Group: cutting straight to its children would drop
// the Group's Occupancy and its toggle, which is exactly what an Absent optional section
// needs to show.
function scrollLayout(plan: Partition): HTMLElement {
  const page = el("div", "form-page");
  for (const node of plan.loose) page.append(renderNode(node, 1));
  for (const section of plan.sections) page.append(renderGroup(section.node, 1));
  return page;
}

// `sections`: a section list beside one section's fields (master-detail). Free navigation —
// every section is reachable at any time, and nothing is withheld for a missing or invalid
// value (ADR 0004 decision 5).
function sectionsLayout(plan: Partition): HTMLElement {
  const shell = el("div", "form-shell");
  const list = el("nav", "section-list");
  const detail = el("div", "section-detail");

  const show = (section: Section): void => {
    detail.replaceChildren(renderGroup(section.node, 1));
    for (const b of list.querySelectorAll("button")) {
      b.setAttribute("aria-current", String(b.dataset.section === section.key));
    }
  };

  for (const section of plan.sections) {
    const button = el("button", "section-tab") as HTMLButtonElement;
    button.type = "button";
    button.dataset.section = section.key;
    button.textContent = section.title;
    const count = violationsIn(section.node).length;
    if (count > 0) button.append(badge(String(count), "violation-count"));
    button.addEventListener("click", () => show(section));
    list.append(button);
  }

  if (plan.loose.length > 0) {
    const loose = el("div", "form-loose");
    for (const node of plan.loose) loose.append(renderNode(node, 1));
    shell.append(loose);
  }
  shell.append(list, detail);
  if (plan.sections.length > 0) show(plan.sections[0]);
  // `reveal` needs to reach a section by key; the layout owns which one is showing.
  if (current) {
    current.show = (key) => {
      const section = plan.sections.find((s) => s.key === key);
      if (section) show(section);
    };
  }
  return shell;
}

function renderNode(node: FormNode, depth: number): HTMLElement {
  switch (node.kind) {
    case "group":
      return renderGroup(node, depth);
    case "repeat":
      return renderRepeat(node, depth);
    case "field":
      return renderField(node);
    case "unknown": {
      // A preserved key the Schema never mentioned. Informational, never a rule failure.
      const row = el("div", "node unknown");
      row.dataset.path = pathText(node.path);
      row.append(label(pathText(node.path)), note(t("form.unknown.preserved"), "notice"));
      const preview = el("pre", "raw-preview");
      preview.textContent = node.rawPreview;
      row.append(preview);
      return row;
    }
    case "cyclic": {
      const row = el("div", "node cyclic");
      row.dataset.path = pathText(node.path);
      row.append(label(pathText(node.path)), note(t("form.cyclic.stopped"), "notice"));
      return row;
    }
  }
}

function renderGroup(node: GroupNode, depth: number): HTMLElement {
  const box = el("section", "node group");
  box.dataset.path = pathText(node.path);
  box.dataset.occupancy = node.occupancy;
  box.append(groupHeading(node, depth));
  const toggle = node.toggle;
  if (toggle) {
    const control = el("button", "group-toggle") as HTMLButtonElement;
    control.type = "button";
    control.dataset.enabled = String(toggle.enabled);
    control.textContent = toggle.enabled ? t("form.group.off") : t("form.group.on");
    control.addEventListener("click", () => active.toggleGroup(node.path, !toggle.enabled));
    box.append(control);
  }
  box.append(...chrome(node.meta));
  // An Absent optional Group has no children to walk until it is turned on.
  for (const child of node.children) box.append(renderNode(child, depth + 1));
  return box;
}

// The cards are `repeat.ts`'s; the walk stays here, so a card's contents are ordinary nodes.
function renderRepeat(node: RepeatNode, depth: number): HTMLElement {
  return renderRepeatCards(node, depth, {
    addItem: (path) => active.addItem(path),
    removeItem: (path, index) => active.removeItem(path, index),
    renderChild: renderNode,
  });
}

// The control is the registry's; the row is this file's. A Field is always a labelled row
// even when the Widget offers nothing to write, so an unwritable value is never invisible.
function renderField(node: FieldNode): HTMLElement {
  const row = el("div", "node field");
  row.dataset.path = pathText(node.path);
  row.dataset.widget = node.widget;
  if (node.widget !== node.intended) row.dataset.intended = node.intended;

  const name = label(node.meta.title || pathText(node.path));
  if (node.meta.required) name.append(badge("*", "required-marker"));
  if (node.meta.deprecated) name.append(badge(t("form.badge.deprecated"), "deprecated"));
  if (node.meta.locked) name.append(badge(t("form.badge.locked"), "locked"));
  row.append(name);

  const control = mount(node, active);
  // `.invalid` is the value's state; `.violation` is reserved for the message, so a
  // "what is wrong" sweep of the DOM never counts the field twice.
  if (node.presence.kind === "invalid") control.classList.add("invalid");
  if (node.meta.unit) control.append(badge(node.meta.unit, "unit"));
  row.append(control, ...chrome(node.meta));
  if (node.presence.kind === "invalid") {
    row.append(...node.presence.violations.map((v) => note(v.message, "violation")));
  }
  return row;
}
