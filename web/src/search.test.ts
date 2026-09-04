// Form search is the *compiler's* (presentation §5.3): `confyg_form::search` scores titles,
// descriptions and Paths, and `confyg-form/tests/search.rs` pins that. This module holds the
// one part that cannot live there — moving the Partition to the section containing the hit —
// so these tests cover mapping and jumping, never matching.
//
// It shares no code and no term with the Option filter (§3.1), which filters choices inside
// one Widget.
import { describe, expect, test } from "vitest";
import { partition } from "./partition.js";
import { resultList, sectionFor } from "./search.js";
import snapshot from "./__fixtures__/demo-snapshot.json";
import type { Hit, SetterSnapshot } from "./types.js";

const demo = snapshot as unknown as SetterSnapshot;
const plan = partition(demo.ir);

function hit(path: string, title: string): Hit {
  return {
    path: path.split(".").map((key) => ({ Key: key })),
    title,
    score: 1,
  };
}

describe("search", () => {
  test("a hit inside a section names the section that holds it", () => {
    expect(sectionFor("tls.cert", plan)).toBe("tls");
  });

  test("a hit that sits loose names no section", () => {
    // Free navigation: a loose node is already on the page, so there is nothing to move.
    expect(sectionFor("port", plan)).toBeNull();
    expect(sectionFor("servers[0].host", plan)).toBeNull();
  });

  test("a result row carries its Path and the compiler's own title", () => {
    const el = resultList([hit("tls.cert", "cert")], () => {});
    const row = el.querySelector<HTMLElement>(".search-hit");
    expect(row?.dataset.path).toBe("tls.cert");
    expect(row?.textContent).toContain("cert");
  });

  test("results keep the order the compiler returned", () => {
    // Re-ranking host-side is exactly the drift §5.3 rejects.
    const el = resultList([hit("b", "second"), hit("a", "first")], () => {});
    expect([...el.querySelectorAll<HTMLElement>(".search-hit")].map((r) => r.dataset.path)).toEqual(
      ["b", "a"],
    );
  });

  test("picking a result jumps to its node", () => {
    const jumps: string[] = [];
    const el = resultList([hit("tls.cert", "cert")], (path) => jumps.push(path));
    el.querySelector<HTMLButtonElement>(".search-hit button")?.click();
    expect(jumps).toEqual(["tls.cert"]);
  });

  test("no hits reads as no matches, never as an empty box", () => {
    expect(resultList([], () => {}).textContent).toMatch(/no matches/i);
  });
});
