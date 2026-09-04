// The renderer shell: a recursive Form IR walk dispatching on `kind`, laid out by the
// Partition (presentation §5). New code, not a port — confy's `render.ts` and its `ViewRow`
// serve a tree editor whose row model is a value/type pair, and ADR 0002 keeps the two
// apart.
//
// Field controls come from the widget registry (`widgets/index.ts`); this file owns
// everything around them: sections, headings, group containers, the entry column a Repeat
// section gets, the violation summary, and the Presence-independent chrome.
//
// Redesign notes:
//   - a Field is a three-track grid (label | control | menu) with every cell placed
//     explicitly, so a conditional message can never shift the next row into the wrong
//     column;
//   - a Repeat that *is* a section gets an entry column (choose an entry, then edit it)
//     rather than a stack of cards. `repeat.ts` still renders nested Repeats as cards;
//   - Unknown keys fold to the foot of the page: preserved, never a rule failure, and
//     never in the way of the fields the Schema does describe.
import { badge, chrome, el, groupHeading, label, note, violationsIn } from "./dom.js";
import { t, tArgs } from "./i18n.js";
import { partition, type Partition, type Section } from "./partition.js";
import { renderRepeat as renderRepeatCards } from "./repeat.js";
import { attachLongPress, rowMenuButton, schemaText } from "./rowmenu.js";
import { sectionFor } from "./search.js";
import { shell as appShell } from "./shell.js";
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

let active: HostCtx = {
  set: () => {},
  unset: () => {},
  addItem: () => {},
  removeItem: () => {},
  toggleGroup: () => {},
};

let current: { root: HTMLElement; plan: Partition; show: ((key: string) => void) | null } | null =
  null;

/** Which entry of a Repeat section is showing, by Repeat path. Survives a re-render. */
const chosenEntry = new Map<string, number>();

export function render(snapshot: SetterSnapshot, root: HTMLElement, ctx?: HostCtx): void {
  if (ctx) active = ctx;
  const plan = partition(snapshot.ir);
  current = { root, plan, show: null };
  const body = plan.kind === "sections" ? sectionsLayout(plan) : scrollLayout(plan);
  root.replaceChildren(renderSummary(snapshot.summary, reveal), body);
  root.dataset.partition = plan.kind;
  // The chrome reads the rendered document rather than the snapshot: one source of truth
  // for "what is wrong", and it is the thing the user can actually see.
  appShell.afterRender();
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
  const target = [...current.root.querySelectorAll<HTMLElement>("[data-path]")].find(
    (node) => node.dataset.path === path,
  );
  if (!target) return;
  for (const prior of current.root.querySelectorAll(".revealed")) {
    prior.classList.remove("revealed");
  }
  target.classList.add("revealed");
  // In the pushed shell the section is a page of its own; showing a field means being on it.
  appShell.showDetail();
}

// `scroll`: one page, and a section is its Group's own heading.
function scrollLayout(plan: Partition): HTMLElement {
  const page = el("div", "form-page");
  appendChildren(page, plan.loose, 1);
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
    detail.replaceChildren(...detailFor(section));
    for (const b of list.querySelectorAll("button")) {
      b.setAttribute("aria-current", String(b.dataset.section === section.key));
    }
    appShell.sectionShown(section.title);
  };

  for (const section of plan.sections) {
    const button = el("button", "section-tab") as HTMLButtonElement;
    button.type = "button";
    button.dataset.section = section.key;
    const name = el("span", "section-name");
    name.textContent = section.title;
    button.append(name);
    const count = violationsIn(section.node).length;
    if (count > 0) button.append(badge(String(count), "violation-count"));
    if (section.repeat) {
      const n = el("span", "entry-count");
      n.textContent = String(section.repeat.items.length);
      button.append(n);
    }
    button.addEventListener("click", () => show(section));
    list.append(button);
  }

  if (plan.loose.length > 0) {
    const loose = el("div", "form-loose");
    appendChildren(loose, plan.loose, 1);
    shell.append(loose);
  }
  shell.append(list, detail);
  if (plan.sections.length > 0) show(plan.sections[0]);
  if (current) {
    current.show = (key) => {
      const section = plan.sections.find((s) => s.key === key);
      if (section) show(section);
    };
  }
  return shell;
}

/**
 * A section's detail. A plain Group is itself — cutting to its children would drop its
 * Occupancy and its toggle, which is exactly what an Absent optional section needs to show.
 * A Repeat section is an entry column plus the chosen entry's fields.
 */
function detailFor(section: Section): HTMLElement[] {
  if (!section.repeat) return [renderGroup(section.node, 1)];
  const repeat = section.repeat;
  const key = pathText(repeat.path);
  const index = Math.min(chosenEntry.get(key) ?? 0, Math.max(0, repeat.items.length - 1));
  chosenEntry.set(key, index);

  const head = el("div", "node group");
  head.dataset.path = key;
  head.append(groupHeading(section.node, 1));
  const path = el("span", "node-path");
  path.textContent = key;
  path.title = tArgs("form.repeat.bounds", [String(repeat.items.length)]);
  head.append(path, ...chrome(section.node.meta));

  const column = entryColumn(repeat, index);
  const body = el("div", "entry-body");
  const chosen = repeat.items[index];
  if (chosen) appendChildren(body, childrenOf(chosen), 2);
  else body.append(note(t("form.repeat.empty"), "notice"));
  return [head, column, body];
}

/** The entry list: label from the entry, a marker when the entry is what is wrong. */
function entryColumn(repeat: RepeatNode, index: number): HTMLElement {
  const box = el("div", "entry-column");
  const title = el("p", "entry-column-title");
  title.textContent = repeat.meta.title || pathText(repeat.path);
  const count = el("span", "entry-count");
  count.textContent = "(" + repeat.items.length + ")";
  title.append(count);
  box.append(title);

  repeat.items.forEach((entry, i) => {
    const tab = el("button", "entry-tab") as HTMLButtonElement;
    tab.type = "button";
    tab.setAttribute("aria-current", String(i === index));
    const name = el("span", "entry-label");
    const first = el("span", "entry-name");
    first.textContent = entryLabel(entry, i);
    name.append(first);
    const bad = violationsIn(entry);
    if (bad.length > 0) {
      const why = el("span", "missing");
      why.textContent = bad[0].message;
      name.append(why);
    }
    tab.append(name);
    tab.addEventListener("click", () => {
      chosenEntry.set(pathText(repeat.path), i);
      // Re-render through the host so the choice goes through one path only.
      const detail = tab.closest(".section-detail");
      const plan = current?.plan;
      const section = plan?.sections.find((s) => s.repeat === repeat);
      if (detail && section) detail.replaceChildren(...detailFor(section));
      appShell.closeEntryDrawer();
    });
    box.append(tab);
  });

  // The two bounds gates, directly under the last entry rather than pinned to the foot.
  const actions = el("div", "entry-actions");
  const add = el("button", "add") as HTMLButtonElement;
  add.type = "button";
  add.textContent = t("form.repeat.add");
  add.addEventListener("click", () => active.addItem(repeat.path));
  const remove = el("button", "remove") as HTMLButtonElement;
  remove.type = "button";
  remove.textContent = t("form.repeat.remove");
  remove.addEventListener("click", () => active.removeItem(repeat.path, index));
  remove.disabled = repeat.items.length === 0;
  actions.append(add, remove);
  box.append(actions);
  return box;
}

/** A Repeat entry reads as its own first scalar, and only falls back to an ordinal. */
function entryLabel(entry: FormNode, i: number): string {
  if ("meta" in entry && entry.meta.title) return entry.meta.title;
  const first = childrenOf(entry).find((c) => c.kind === "field") as FieldNode | undefined;
  if (first && first.presence.kind !== "absent") {
    const raw = first.presence.literal;
    try {
      const parsed: unknown = JSON.parse(raw);
      if (typeof parsed === "string" && parsed !== "") return parsed;
    } catch {
      if (raw !== "") return raw;
    }
  }
  return "#" + (i + 1);
}

function childrenOf(node: FormNode): FormNode[] {
  return node.kind === "group" ? node.children : node.kind === "repeat" ? node.items : [];
}

/**
 * Walk a container's children, with Unknown keys held back to a fold at the end. Preserved
 * keys are a fact about the file, not a task: they stay visible and stay out of the way.
 */
function appendChildren(box: HTMLElement, children: FormNode[], depth: number): void {
  const unknown = children.filter((c) => c.kind === "unknown");
  for (const child of children) {
    if (child.kind !== "unknown") box.append(renderNode(child, depth));
  }
  if (unknown.length === 0) return;
  const fold = el("details", "unknown-fold");
  const summary = document.createElement("summary");
  summary.textContent = tArgs("form.unknown.fold", [String(unknown.length)]);
  fold.append(summary);
  for (const child of unknown) fold.append(renderNode(child, depth));
  box.append(fold);
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
  const path = el("span", "node-path");
  path.textContent = pathText(node.path);
  path.title = pathText(node.path) + " \u00b7 " + t("form.occupancy." + node.occupancy);
  box.append(path);
  const toggle = node.toggle;
  if (toggle) {
    const control = el("button", "group-toggle") as HTMLButtonElement;
    control.type = "button";
    control.dataset.enabled = String(toggle.enabled);
    control.setAttribute("aria-pressed", String(toggle.enabled));
    control.title = t("form.group.optional");
    control.textContent = toggle.enabled ? t("form.group.off") : t("form.group.on");
    control.addEventListener("click", () => active.toggleGroup(node.path, !toggle.enabled));
    box.append(control);
  }
  box.append(...chrome(node.meta));
  appendChildren(box, node.children, depth + 1);
  return box;
}

// Nested Repeats stay cards; a Repeat that is a section gets the entry column instead.
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
  // Hover on the label is where the Schema explains itself, on every field without
  // exception — the row menu says the same thing for a finger.
  name.title = schemaText(node);
  if (node.meta.required) name.append(badge("*", "required-marker"));
  if (node.meta.deprecated) name.append(badge(t("form.badge.deprecated"), "deprecated"));
  if (node.meta.locked) name.append(badge(t("form.badge.locked"), "locked"));
  row.append(name);

  const control = mount(node, active);
  if (node.presence.kind === "invalid") control.classList.add("invalid");
  if (node.meta.unit) control.append(badge(node.meta.unit, "unit"));
  row.append(control, rowMenuButton(node, active), ...chrome(node.meta));
  if (node.presence.kind === "invalid") {
    row.append(...node.presence.violations.map((v) => note(v.message, "violation")));
  }
  attachLongPress(row, node, active);
  return row;
}
