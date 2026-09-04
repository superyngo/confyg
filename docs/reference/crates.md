# Crates

The Rust core and the web host as they exist today. Canonical for *what each package owns*; the
reasoning behind the split lives in [the design record](../spec/2026-09-03-confyg-design.md) and
is not restated here.

| Crate | Owns | Never does |
|---|---|---|
| `confyg-form` | `Schema -> Form IR`: keyword introspection, the `x-confyg` Annotation, Widget resolution and the Host clamp, compilation, Document overlay, the unknown sweep, the violation summary, **Form search** | No I/O, no state, no mutations |
| `confyg-session` | Ordinal conversion, intent lowering onto `confy-core` `Mutation`s, fragments, Templates, the session and its undo ring, the D9 postcondition guard | No I/O — an unresolved Schema hint is handed back to the host |
| `confyg-ffi` | Serialization across the WASM boundary: `dispatch`, `check` and `search` | No logic; a native host links `confyg-session` directly |
| `web/` | The renderer: the Partition, the recursive IR walk, Repeat cards, the violation summary, Appearance tokens, and host file I/O | No form decisions of its own — every one arrives in a `SetterSnapshot`, and it never matches a search itself |

## Entry points

- `confyg_form::compile::project(schema, doc, host) -> Compiled` — design §4 in full.
  `compile(schema, host)` is `project(schema, None, host)`: the all-**Absent** tree.
- `confyg_form::unknown::summary(&root, &state) -> Summary` — every attributed **Violation**, plus
  whether the document could be validated at all.
- `confyg_form::search::search(&root, query) -> Vec<Hit>` — fuzzy matching over node titles,
  descriptions and **Paths**, ranked best-first, ties broken on Path. Presentation §5.3 puts it
  here rather than in each host: a hit has to be able to move the **Partition**, and two
  host-side matchers drift. `path_text` renders a Path the way `web/src/types.ts` `pathText`
  does — the same string a host addresses a node by.
- `confyg_session::session::Session::dispatch(Request) -> SetterSnapshot` — the only call a host
  needs. `SetterSnapshot` is the only type crossing the FFI boundary.
- `confyg_session::session::Session::check(path, literal) -> Vec<Violation>` — a live check for an
  uncommitted buffer, through the *validator's* engine so warnings cannot disagree with it.
- `confyg_session::session::Session::search(query) -> Vec<Hit>` — the same call against the
  document as it stands. It recompiles, exactly as `check` does, rather than publishing its IR:
  a public accessor would let a host read the compiled tree and decide things locally.
- `confyg_ffi::{dispatch, check, search}` — the same three calls, JSON in, JSON out. `search`
  takes the query as raw text, not JSON.
- `web/src/render.ts` `render(snapshot, root, ctx)` — the whole host-side entry point, and
  `reveal(path)`, which a summary item or a search hit jumps through.
  `web/src/partition.ts` `partition(ir)` cuts the tree into screens (presentation §5.1); fewer
  than three depth-1 **Groups** falls back to `scroll`.
- `web/src/widgets/index.ts` `mount(field, ctx) -> HTMLElement` — the control for one Field,
  from a `Record<Widget, Mount>` keyed by the closed vocabulary, so an unmapped Widget is a
  build error. A **Locked** or `readOnly` field mounts as display only whatever its Widget.
  `ctx.set` takes a **JSON literal** — `rawText` alone hands over bytes as authored.
- `web/src/repeat.ts` `renderRepeat(node, depth, ctx)` — a card per object entry, a row per
  scalar one, and the two bounds gates. The gates are the same comparisons `lower.rs` refuses
  on, so the host never offers an intent the core would decline; a card's `data-index` is an
  **entry** index.
- `web/src/summary.ts` `renderSummary(summary, jump)` and `web/src/search.ts`
  `{ sectionFor, resultList }` — the summary above the form, and the section a hit belongs to.
  Neither matches anything: the ranking arrives from `confyg_form::search`.
- `web/src/dom.ts` — the chrome builders `render.ts`, `repeat.ts`, `summary.ts` and `search.ts`
  share. Extracted when the fourth caller appeared; the alternative was an import cycle.

## Index spaces, restated once

The Form IR counts **entries**; `Target.index` counts **children**, comments included.
`confyg_session::ordinal` is the only place that converts between them, and
[`upstream.md`](upstream.md) *Index spaces* is the canonical description of why. The same module
answers where an absent key belongs: `schema_slot` for a plain key, `header_slot` for a
section — TOML clamps the two in opposite directions (*Insertion legality*).

## Gates

Five CI jobs, each pinning an invariant rather than a style:

- `rust` — the workspace test suite, plus a grep that fails if `confy_core::session` is ever
  referenced *in code* (ADR 0001). The pattern skips comment lines, because the module docs
  discuss the ban in prose.
- `write-neutrality` — presentation input may never change a written byte (ADR 0004).
- `wasm` — the boundary still builds for `wasm32-unknown-unknown`.
- `web` — the renderer typechecks, its tests pass, and the production bundle builds. Stays the
  sub-minute gate: no browser, no WASM.
- `e2e` — the flow, on the real build (design §11 item 5). Builds the WASM, builds the bundle,
  drives Chromium. Its own job rather than a step in `web` for the obvious reason.

## Tests worth knowing about

| File | Pins |
|---|---|
| `confyg-form/tests/snapshot.rs` | The real `eslintrc.json` outline on two Host profiles, and one identical outline for the same document in all three **Doc formats** |
| `confyg-session/tests/roundtrip.rs` | `(schema, document, [intent]) -> exact bytes`, every case in three formats, comment-interleaved |
| `confyg-session/tests/postcondition.rs` | Every intent recompiles to the shape it predicted (D9) |
| `confyg-session/tests/write_neutrality.rs` | Eleven Annotations x four Host profiles x three formats produce identical bytes |
| `confyg-form/tests/search.rs` | Form search's three axes — a title and a description that its key never says, and the Path — plus an empty query finding nothing |
| `confyg-ffi/tests/boundary.rs` | The exact intent JSON the web host builds, `addRepeatItem` / `removeRepeatItem` / `toggleGroup` included, and `search` as raw text |
| `web/src/partition.test.ts` | The `sections` floor, and that only depth-1 Groups become sections |
| `web/src/render.test.ts` | The shell walked over a snapshot the real core produced (`--example try`, `json`) |
| `web/src/widgets/presence.test.ts` | Every Widget reaches all three Presence states, a boolean is three-state, a Locked node has no write affordance |
| `web/src/repeat.test.ts` | The count badge, both bounds gates at the fixture's own ceiling and floor, cards vs scalar rows, and the entry index a card publishes |
| `web/src/summary.test.ts` | An uncompilable Schema reads *validation unavailable*, never *no problems* |
| `web/src/search.test.ts` | A hit maps to the section holding it, and the host preserves the compiler's order |
| `tests/e2e/first-run.spec.ts` | The whole flow on the built artifact — open → add a Repeat item → set values → Unset one → save — asserting the *emitted bytes* (design §11 item 5) |

One test is `#[ignore]`d on purpose: `a_delete_keeps_the_blank_line_that_separates_a_comment_block`
asserts the bytes upstream bill item 3 will make possible. `cargo test -- --ignored` shows what is
still owed.

`cargo run -p confyg-session --example try -- <schema> <config>` drives the whole write path from
a terminal — `find <query>` runs **Form search** the same way — which is how the two findings in
[`../debug/2026-09-04-phase-a-hands-on-findings.md`](../debug/2026-09-04-phase-a-hands-on-findings.md)
were found.

`wasm-pack build crates/confyg-ffi --target web && npm run test:e2e` runs the real-binary check.
The WASM build is a *prior* step, never inside Playwright's `webServer`, which builds and
previews the bundle itself. That run is how the three findings in
[`../debug/2026-09-04-real-binary-findings.md`](../debug/2026-09-04-real-binary-findings.md)
were found — none of them visible to any test above it, which is the argument for the job.
