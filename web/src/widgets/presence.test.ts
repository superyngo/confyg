// The contract every widget owes: all three Presence states are reachable in all of them, a
// boolean is three-state, a Locked node has no write affordance, and a clamped Widget says so.
import { describe, expect, test } from "vitest";
import { ALL_WIDGETS, mount, type Ctx } from "./index.js";
import type { FieldMeta, FieldNode, Widget } from "../types.js";

const NOOP: Ctx = { set: () => {}, unset: () => {} };

function meta(over: Partial<FieldMeta> = {}): FieldMeta {
  return {
    title: "Mode",
    description: null,
    violations: [],
    locked: null,
    deprecated: false,
    default: null,
    examples: [],
    required: false,
    readOnly: false,
    writeOnly: false,
    unit: null,
    constraints: [],
    raw: false,
    options: [],
    ...over,
  };
}

function field(widget: Widget, over: Partial<FieldNode> = {}): FieldNode {
  return {
    kind: "field",
    path: [{ Key: "mode" }],
    widget,
    intended: widget,
    presence: { kind: "absent", default: "info", remarked: null },
    meta: meta({ default: "info", options: [{ value: "info", label: "info" }] }),
    ...over,
  };
}

const writableControls = (el: HTMLElement): HTMLElement[] => [
  ...el.querySelectorAll<HTMLElement>("input, select, textarea, button"),
];

describe("presence", () => {
  test("every widget shows the default it would inherit", () => {
    for (const w of ALL_WIDGETS) {
      const el = mount(field(w), NOOP);
      expect(el.textContent, w).toContain("info");
    }
  });

  test("every widget offers a way back to Absent", () => {
    for (const w of ALL_WIDGETS) {
      if (w === "displayOnly") continue; // nothing to unset: it is never writable
      const el = mount(field(w, { presence: { kind: "set", literal: '"warn"' } }), NOOP);
      const inherited = el.querySelector('option[value="inherited"], input[value="inherited"]');
      const unset = el.querySelector<HTMLButtonElement>("button.unset");
      expect(inherited ?? unset, `${w} offers neither an inherited option nor an Unset button`)
        .toBeTruthy();
      // Whichever it is, it must be usable once the value is written.
      if (unset) expect(unset.disabled, w).toBe(false);
    }
  });

  test("a boolean is three-state, never a bare checkbox", () => {
    const el = mount(field("tristate", { meta: meta({ default: false }) }), NOOP);
    const values = [...el.querySelectorAll("option")].map((o) => o.value);
    expect(values).toEqual(["inherited", "true", "false"]); // ADR 0002
    expect(el.querySelector('input[type="checkbox"]')).toBeNull();
  });

  test("a locked node has no write affordance", () => {
    const locked = field("text", { meta: meta({ locked: { reason: "yamlAlias" } }) });
    expect(writableControls(mount(locked, NOOP))).toHaveLength(0);
  });

  test("a readOnly field has no write affordance either, whatever its Widget", () => {
    const ro = field("slider", { meta: meta({ readOnly: true }) });
    expect(writableControls(mount(ro, NOOP))).toHaveLength(0);
  });

  test("a clamped widget explains itself", () => {
    const clamped = field("text", { intended: "masked" });
    const note = mount(clamped, NOOP).querySelector(".degrade");
    expect(note?.textContent).toContain("Masked"); // presentation §6: the Chassis lexicon
  });

  test("an unclamped widget stays silent about it", () => {
    expect(mount(field("text"), NOOP).querySelector(".degrade")).toBeNull();
  });

  test("an Absent field with no default says so rather than showing nothing", () => {
    const bare = field("text", {
      presence: { kind: "absent", default: null, remarked: null },
      meta: meta(),
    });
    expect(mount(bare, NOOP).querySelector(".ghost")?.textContent).toBe("not set");
  });

  test("a menu seeds itself from the literal, not from the label", () => {
    const set = field("menu", {
      presence: { kind: "set", literal: '"warn"' },
      meta: meta({
        options: [
          { value: "info", label: "Info" },
          { value: "warn", label: "Warnings only" },
        ],
      }),
    });
    const select = mount(set, NOOP).querySelector("select") as HTMLSelectElement;
    expect(select.value).toBe('"warn"');
  });
});
