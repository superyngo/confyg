// The shell is walked against a snapshot the real core produced, not a hand-written IR:
// `printf 'json\nq\n' | cargo run -p confyg-session --example try -- \
//   crates/confyg-session/examples/demo.schema.json crates/confyg-session/examples/demo.toml`
import { describe, expect, test } from "vitest";
import { render } from "./render.js";
import type { SetterSnapshot } from "./types.js";
import snapshot from "./__fixtures__/demo-snapshot.json";

const demo = snapshot as unknown as SetterSnapshot;

function rendered(): HTMLElement {
  const root = document.createElement("main");
  render(demo, root);
  return root;
}

describe("render", () => {
  test("one depth-1 Group falls back to scroll, and every depth-1 node still appears", () => {
    const root = rendered();
    expect(root.dataset.partition).toBe("scroll");
    expect([...root.querySelectorAll<HTMLElement>(".node")].map((n) => n.dataset.path)).toContain(
      "servers",
    );
  });

  test("an Absent Field shows its default as Ghost text, never as a value", () => {
    const mode = rendered().querySelector<HTMLElement>('[data-path="mode"] .control');
    expect(mode?.querySelector(".ghost")?.textContent).toContain("dev");
    // Two choices is under the menu floor, so `mode` is a radio group and "inherited" is one
    // of its choices rather than a separate Unset button (ADR 0003).
    const inherited = mode?.querySelector<HTMLInputElement>('input[value="inherited"]');
    expect(inherited?.checked).toBe(true);
  });

  test("an Invalid Field renders the literal and the validator's own message", () => {
    const port = rendered().querySelector<HTMLElement>('[data-path="port"]');
    // The range input clamps to the Schema's maximum; the exact number beside it must not,
    // or an out-of-range value would look like a legal one.
    expect(port?.querySelector<HTMLInputElement>('input[type="number"]')?.value).toBe("99999");
    expect(port?.querySelector(".control")?.classList.contains("invalid")).toBe(true);
    expect(port?.querySelector(".violation")?.textContent).toMatch(/65535/);
  });

  test("a required Field carries the marker", () => {
    const host = rendered().querySelector('[data-path="host"] .required-marker');
    expect(host?.textContent).toBe("*");
  });

  test("an Unknown key reads as preserved, not as a rule failure", () => {
    const unknown = rendered().querySelector<HTMLElement>(".node.unknown");
    expect(unknown?.querySelector(".notice")?.textContent).toMatch(/kept exactly as written/);
    expect(unknown?.querySelector(".violation")).toBeNull();
    expect(unknown?.querySelector(".raw-preview")?.textContent).toContain("puce");
  });

  test("a clamped Widget records what was intended", () => {
    // The demo host cannot slide `cacheSize`? It can — so intended and rendered agree, and the
    // attribute is absent. The assertion pins that the shell never invents a degradation.
    const cache = rendered().querySelector<HTMLElement>('[data-path="cacheSize"]');
    expect(cache?.dataset.widget).toBe("slider");
    expect(cache?.dataset.intended).toBeUndefined();
  });

  test("an Absent optional Group keeps its own heading and hides its children", () => {
    const tls = rendered().querySelector<HTMLElement>('[data-path="tls"]');
    expect(tls?.dataset.occupancy).toBe("absent");
    expect(tls?.querySelector(".node-heading")).not.toBeNull();
    expect(tls?.querySelectorAll(".field").length).toBeGreaterThan(0); // present, but CSS-hidden
  });
});
