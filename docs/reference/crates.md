# Crates

The Rust core and the web host as they exist today. Canonical for *what each package owns*; the
reasoning behind the split lives in [the design record](../spec/2026-09-03-confyg-design.md) and
is not restated here.

| Crate | Owns | Never does |
|---|---|---|
| `confyg-form` | `Schema -> Form IR`: keyword introspection, the `x-confyg` Annotation, Widget resolution and the Host clamp, compilation, Document overlay, the unknown sweep, the violation summary | No I/O, no state, no mutations |
| `confyg-session` | Ordinal conversion, intent lowering onto `confy-core` `Mutation`s, fragments, Templates, the session and its undo ring, the D9 postcondition guard | No I/O — an unresolved Schema hint is handed back to the host |
| `confyg-ffi` | Serialization across the WASM boundary: `dispatch` and `check` | No logic; a native host links `confyg-session` directly |
| `web/` | The renderer: the Partition, the recursive IR walk, Appearance tokens, and host file I/O | No form decisions of its own — every one arrives in a `SetterSnapshot` |

## Entry points

- `confyg_form::compile::project(schema, doc, host) -> Compiled` — design §4 in full.
  `compile(schema, host)` is `project(schema, None, host)`: the all-**Absent** tree.
- `confyg_form::unknown::summary(&root, &state) -> Summary` — every attributed **Violation**, plus
  whether the document could be validated at all.
- `confyg_session::session::Session::dispatch(Request) -> SetterSnapshot` — the only call a host
  needs. `SetterSnapshot` is the only type crossing the FFI boundary.
- `confyg_session::session::Session::check(path, literal) -> Vec<Violation>` — a live check for an
  uncommitted buffer, through the *validator's* engine so warnings cannot disagree with it.
- `confyg_ffi::{dispatch, check}` — the same two calls, JSON in, JSON out.
- `web/src/render.ts` `render(snapshot, root)` — the whole host-side entry point.
  `web/src/partition.ts` `partition(ir)` cuts the tree into screens (presentation §5.1); fewer
  than three depth-1 **Groups** falls back to `scroll`.

## Index spaces, restated once

The Form IR counts **entries**; `Target.index` counts **children**, comments included.
`confyg_session::ordinal` is the only place that converts between them, and
[`upstream.md`](upstream.md) *Index spaces* is the canonical description of why. The same module
answers where an absent key belongs: `schema_slot` for a plain key, `header_slot` for a
section — TOML clamps the two in opposite directions (*Insertion legality*).

## Gates

Four CI jobs, each pinning an invariant rather than a style:

- `rust` — the workspace test suite, plus a grep that fails if `confy_core::session` is ever
  referenced (ADR 0001).
- `write-neutrality` — presentation input may never change a written byte (ADR 0004).
- `wasm` — the boundary still builds for `wasm32-unknown-unknown`.
- `web` — the renderer typechecks, its tests pass, and the production bundle builds.

## Tests worth knowing about

| File | Pins |
|---|---|
| `confyg-form/tests/snapshot.rs` | The real `eslintrc.json` outline on two Host profiles, and one identical outline for the same document in all three **Doc formats** |
| `confyg-session/tests/roundtrip.rs` | `(schema, document, [intent]) -> exact bytes`, every case in three formats, comment-interleaved |
| `confyg-session/tests/postcondition.rs` | Every intent recompiles to the shape it predicted (D9) |
| `confyg-session/tests/write_neutrality.rs` | Eleven Annotations x four Host profiles x three formats produce identical bytes |
| `web/src/partition.test.ts` | The `sections` floor, and that only depth-1 Groups become sections |
| `web/src/render.test.ts` | The shell walked over a snapshot the real core produced (`--example try`, `json`) |

One test is `#[ignore]`d on purpose: `a_delete_keeps_the_blank_line_that_separates_a_comment_block`
asserts the bytes upstream bill item 3 will make possible. `cargo test -- --ignored` shows what is
still owed.

`cargo run -p confyg-session --example try -- <schema> <config>` drives the whole write path from
a terminal, which is how the two findings in
[`../debug/2026-09-04-phase-a-hands-on-findings.md`](../debug/2026-09-04-phase-a-hands-on-findings.md)
were found.
