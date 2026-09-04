# Crates

The Rust core as it exists today. Canonical for *what each crate owns*; the reasoning behind the
split lives in [the design record](../spec/2026-09-03-confyg-design.md) and is not restated here.

| Crate | Owns | Never does |
|---|---|---|
| `confyg-form` | `Schema -> Form IR`: keyword introspection, the `x-confyg` Annotation, Widget resolution and the Host clamp, compilation, Document overlay, the unknown sweep, the violation summary | No I/O, no state, no mutations |
| `confyg-session` | Ordinal conversion, intent lowering onto `confy-core` `Mutation`s, fragments, Templates, the session and its undo ring, the D9 postcondition guard | No I/O — an unresolved Schema hint is handed back to the host |
| `confyg-ffi` | Serialization across the WASM boundary: `dispatch` and `check` | No logic; a native host links `confyg-session` directly |

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

## Index spaces, restated once

The Form IR counts **entries**; `Target.index` counts **children**, comments included.
`confyg_session::ordinal` is the only place that converts between them, and
[`upstream.md`](upstream.md) *Index spaces* is the canonical description of why.

## Gates

Three CI jobs, each pinning an invariant rather than a style:

- `rust` — the workspace test suite, plus a grep that fails if `confy_core::session` is ever
  referenced (ADR 0001).
- `write-neutrality` — presentation input may never change a written byte (ADR 0004).
- `wasm` — the boundary still builds for `wasm32-unknown-unknown`.

## Tests worth knowing about

| File | Pins |
|---|---|
| `confyg-form/tests/snapshot.rs` | The real `eslintrc.json` outline on two Host profiles, and one identical outline for the same document in all three **Doc formats** |
| `confyg-session/tests/roundtrip.rs` | `(schema, document, [intent]) -> exact bytes`, every case in three formats, comment-interleaved |
| `confyg-session/tests/postcondition.rs` | Every intent recompiles to the shape it predicted (D9) |
| `confyg-session/tests/write_neutrality.rs` | Eleven Annotations x four Host profiles x three formats produce identical bytes |
