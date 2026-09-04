// The violation summary: design §11's C6 item, rendered. The list itself comes from the
// snapshot the real core produced; only the `unavailable` verdict is written by hand, because
// a `Summary` is a two-field record the core serializes verbatim and
// `confyg-form/tests/compile.rs` already pins that a bad `pattern` produces it.
import { describe, expect, test } from "vitest";
import { renderSummary } from "./summary.js";
import snapshot from "./__fixtures__/demo-snapshot.json";
import type { SetterSnapshot, Summary } from "./types.js";

const demo = snapshot as unknown as SetterSnapshot;

function text(summary: Summary): string {
  return renderSummary(summary, () => {}).textContent ?? "";
}

describe("summary", () => {
  test("an uncompilable Schema says validation unavailable, never no problems", () => {
    const el = renderSummary(
      { items: [], validation: { kind: "unavailable", keyword: "pattern", pointer: "#/a" } },
      () => {},
    );
    expect(el.textContent).toMatch(/unavailable/i);
    expect(el.textContent).not.toMatch(/no problems/i);
    // D8: the document loses validation, not its form — so this is a notice, not a failure.
    expect(el.querySelectorAll(".summary-item")).toHaveLength(0);
  });

  test("a validated document with nothing wrong says so", () => {
    expect(text({ items: [], validation: { kind: "available" } })).toMatch(/no problems/i);
  });

  test("the real snapshot's own violation is listed with the validator's message", () => {
    const el = renderSummary(demo.summary, () => {});
    const items = el.querySelectorAll<HTMLElement>(".summary-item");
    expect(items).toHaveLength(1);
    expect(items[0].textContent).toContain("Port");
    expect(items[0].textContent).toMatch(/65535/);
  });

  test("a summary item jumps to its node", () => {
    const jumps: string[] = [];
    const el = renderSummary(demo.summary, (path) => jumps.push(path));
    el.querySelector<HTMLButtonElement>(".summary-item button")?.click();
    expect(jumps).toEqual(["port"]);
  });
});
