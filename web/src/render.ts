// The renderer shell: a recursive Form IR walk dispatching on `kind`, laid out by the
// Partition (presentation §5). New code, not a port — confy's `render.ts` and its `ViewRow`
// serve a tree editor whose row model is a value/type pair, and ADR 0002 keeps the two
// apart.
//
// Field controls are Task 18's widget registry; this file owns everything around them:
// sections, headings, group and repeat containers, and the Presence-independent chrome
// (required marker, deprecated badge, description, violations).
import { partition, type Partition, type Section } from "./partition.js";
import { t } from "./i18n.js";
import {
  pathText,
  type FieldNode,
  type FormNode,
  type GroupNode,
  type NodeMeta,
  type RepeatNode,
  type SetterSnapshot,
  type Violation,
} from "./types.js";

export function render(snapshot: SetterSnapshot, root: HTMLElement): void {
  const plan = partition(snapshot.ir);
  root.replaceChildren(
    plan.kind === "sections" ? sectionsLayout(plan) : scrollLayout(plan),
  );
  root.dataset.partition = plan.kind;
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
      row.append(label(pathText(node.path)), note(t("form.unknown.preserved"), "notice"));
      const preview = el("pre", "raw-preview");
      preview.textContent = node.rawPreview;
      row.append(preview);
      return row;
    }
    case "cyclic": {
      const row = el("div", "node cyclic");
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
  if (node.toggle) {
    const toggle = el("button", "group-toggle") as HTMLButtonElement;
    toggle.type = "button";
    toggle.dataset.enabled = String(node.toggle.enabled);
    toggle.textContent = node.toggle.enabled ? t("form.group.off") : t("form.group.on");
    box.append(toggle);
  }
  box.append(...chrome(node.meta));
  // An Absent optional Group has no children to walk until it is turned on.
  for (const child of node.children) box.append(renderNode(child, depth + 1));
  return box;
}

// The card layout, the count badge, and the bounds gating are Task 19; the shell walks the
// items so a Repeat is never invisible before then.
function renderRepeat(node: RepeatNode, depth: number): HTMLElement {
  const box = el("section", "node repeat");
  box.dataset.path = pathText(node.path);
  box.dataset.occupancy = node.occupancy;
  box.append(groupHeading(node, depth), ...chrome(node.meta));
  for (const item of node.items) box.append(renderNode(item, depth + 1));
  return box;
}

// The control itself arrives with the widget registry (Task 18). Until then the field is
// rendered read-only — a shell that shows the wrong value would be worse than one that
// shows no affordance.
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

  const value = el("div", "field-value");
  switch (node.presence.kind) {
    case "absent":
      value.classList.add("ghost");
      value.textContent = node.presence.default === undefined || node.presence.default === null
        ? t("form.presence.unset")
        : JSON.stringify(node.presence.default);
      break;
    case "set":
      value.textContent = node.presence.literal;
      break;
    case "invalid":
      // `.invalid` is the value's state; `.violation` is reserved for the message, so a
      // "what is wrong" sweep of the DOM never counts the field twice.
      value.classList.add("invalid");
      value.textContent = node.presence.literal;
      break;
  }
  if (node.meta.unit) value.append(badge(node.meta.unit, "unit"));
  row.append(value, ...chrome(node.meta));
  if (node.presence.kind === "invalid") {
    row.append(...node.presence.violations.map((v) => note(v.message, "violation")));
  }
  return row;
}

// Description and violations: the same chrome for every node kind.
function chrome(meta: NodeMeta): HTMLElement[] {
  const parts: HTMLElement[] = [];
  if (meta.description) parts.push(note(meta.description, "description"));
  parts.push(...meta.violations.map((v) => note(v.message, "violation")));
  return parts;
}

function violationsIn(node: FormNode): Violation[] {
  const own = "meta" in node ? node.meta.violations : [];
  const kids = node.kind === "group" ? node.children : node.kind === "repeat" ? node.items : [];
  return kids.reduce<Violation[]>((all, kid) => all.concat(violationsIn(kid)), [...own]);
}

function groupHeading(node: GroupNode | RepeatNode, depth: number): HTMLElement {
  const h = el(`h${Math.min(depth + 1, 6)}`, "node-heading");
  h.textContent = node.meta.title || pathText(node.path);
  return h;
}

function el(tag: string, cls: string): HTMLElement {
  const node = document.createElement(tag);
  node.className = cls;
  return node;
}

function label(text: string): HTMLElement {
  const node = el("span", "field-label");
  node.textContent = text;
  return node;
}

function note(text: string, cls: string): HTMLElement {
  const node = el("p", cls);
  node.textContent = text;
  return node;
}

function badge(text: string, cls: string): HTMLElement {
  const node = el("span", `badge ${cls}`);
  node.textContent = text;
  return node;
}
