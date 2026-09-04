# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- 2026-09-03 — Repository initialized: documentation layout per `wens-dev-principles docs`
  (root `CONTEXT.md` index, `docs/{reference,adr,spec,plan,debug,audit,tmp}/` with indexes),
  `docs/reference/glossary.md` as the canonical vocabulary, ADR 0001 (`confy-core` as a pinned
  git dependency with `session` feature-gated off) and ADR 0002 (Schema-driven projection and
  three-state Presence), and the design spec `docs/spec/2026-09-03-confyg-design.md`.
- 2026-09-04 — Design grilled against the upstream code and revised: added
  `docs/reference/upstream.md` as the canonical record of `confy-core`'s pin, reachable API,
  index spaces, and the two upstream changes confyg requires; added ADR 0003 (Presence stays a
  Field concept while containers carry Occupancy, a scalar array is a Repeat group, setting a
  value equal to the default is Unset); split Setter intents from Session commands; added Raw
  literal fallback, Comment policy, Locked nodes, Notices, and the Union field; collapsed
  `additionalProperties`' three meanings into one decision table; deferred `MoveRepeatItem` and
  added Draft 7 normalization at v0.2; widened verification to an intent × node kind × format
  matrix. Corrected six factual claims about confy, including a dependency pin that did not
  resolve; ADR 0001 carries an erratum rather than being rewritten.
- 2026-09-04 — Presentation architecture decided after grilling the design record: added ADR 0004
  (eight presentation layers — Value contract, structure, Affordance, Flow, Lexicon, Appearance,
  Emission style, Conduct — with the last two closed to any override; **Write-neutrality** as a
  tested invariant; derivation thresholds fixed while their results stay overridable; Flow split
  into Partition and Traversal and forbidden from gating on validity; host capability declared as
  pure data and clamped at compile time along a Degradation ladder) and ADR 0005 (one closed
  Presentation vocabulary with two carriers — an `x-confyg` Annotation object and an optional
  Presentation profile sidecar keyed by Schema pointer — a fixed resolution chain from built-in
  derivation through to downgrade-only user preference, discovery mirroring §6, no `hidden`, and
  unknown profile keys as Notices; supersedes the glossary's rejection of a separate UI-schema
  file, with the sidecar itself scheduled for v0.2).
- 2026-09-04 — Added `docs/spec/2026-09-04-presentation-layers-design.md`: the eight-layer table
  and resolution chain, the nine-member Presentation vocabulary, Widget resolution with the
  Option filter thresholds, `HostProfile` and a Degradation ladder per Widget, Partition's closed
  four values with `sections` falling back to `scroll` below three sections, Traversal's
  no-validity-gate rule with unfilled/violation counts, Form search as a `confyg-form` pure
  function, Lexicon's split between chassis and Schema-content strings, the added Appearance
  tokens, the Write-neutrality property test, and the presentation tiers. Extended the glossary
  with 15 terms and rewrote the Annotation entry, which now names `x-confyg` as the sole
  annotation and the Presentation profile as its second carrier.
- 2026-09-04 — Reconciled `docs/spec/2026-09-03-confyg-design.md` with the presentation layers:
  `FieldMeta` gains `intended` (the pre-clamp Widget); Widget resolution cites the Affordance
  layer and Degradation ladder rather than restating a precedence list; `x-order` became
  `x-confyg.order` and the hardcoded `label_from` key list became `x-confyg.labelFrom` with that
  list as its derivation default; A5 carries the menu-family thresholds and A4 states that
  masking degrades with a Notice rather than silently; the release tiers absorb the presentation
  work; verification gained a Host capability axis, ladder-terminal coverage, and
  Write-neutrality's property test; the Annotation open question is resolved.
- 2026-09-04 — Added `docs/plan/2026-09-04-confyg-design.md`: the v0.1 implementation plan, 20
  tasks across a Rust core phase (workspace and pin, Form IR types, Schema introspection,
  Presentation vocabulary, Affordance and clamp, compilation, overlay, unknown sweep, ordinal
  conversion, intent lowering, Templates, session dispatch, the D9 postcondition guard, the
  Write-neutrality property test, the WASM boundary) and a web-host phase (renderer shell,
  widget set, Repeat cards and summary and Form search, real-binary check). Every task is
  test-first and ends with a commit; Tasks 9, 14, and 15 are ordered early because D7, D9, and
  Write-neutrality are the failures that report success.
- 2026-09-04 — Scaffolded the Rust workspace on a pinned `confy-core`
  (`rev = 558bee7f`, `default-features = false`) with `serde_json`'s `preserve_order` enabled
  workspace-wide per design §2, added `crates/confyg-form` and a byte-identical TOML round-trip
  test through the public API, and added `.github/workflows/ci.yml` with the gate that fails if
  `confy_core::session` is ever referenced. Corrected `docs/reference/upstream.md`: `confy-core`
  declares no Cargo features on `main`, so `default-features = false` is a forward-compatible
  no-op and the grep gate is what enforces ADR 0001; recorded the real import paths
  (`model::any_doc::AnyDocument`, `model::document::{ConfigDocument, DocFormat}`).
- 2026-09-04 — Defined the Form IR types in `crates/confyg-form/src/ir.rs`: `FormNode`
  (Field / Group / Repeat / Unknown / Cyclic), three-state `Presence`, `Occupancy`, the closed
  twelve-member `Widget` vocabulary, `NodeMeta`, `FieldMeta`, `Bounds`, `GroupToggle`,
  `TemplateRef`, `Locked`, `Constraint`, and `SchemaState` with its `SchemaCompileError`.
  Serialization is externally tagged on `kind` and camelCase throughout, so `web/` and `insta`
  read the same shape.
- 2026-09-04 — Added `crates/confyg-form/src/facts.rs`: one non-panicking pass over a Schema
  object producing `SchemaFacts` — the keyword set `schema::hints_edit` skips (`default`,
  `examples`, `required`, `deprecated`, `readOnly`, `writeOnly`, `prefixItems`,
  `additionalProperties`) plus types, enum/const, bounds, lengths, `pattern`, `multipleOf` and
  `uniqueItems`. `AdditionalProperties::{Schema, Open, Closed}` encodes design §7's three-form
  table; a malformed keyword reads as absent so a broken Schema still projects a complete form.
- 2026-09-04 — Added `crates/confyg-form/src/vocab.rs` and `notice.rs`: the nine-member
  Presentation vocabulary (`affordance`, `order`, `unit`, `collapsed`, `demoted`, `label`,
  `help`, `labelFrom`, `optionLabels`) parsed off the `x-confyg` Annotation, plus
  `profile_hint` for the root-only Profile hint. An unknown or wrongly-typed member is a
  `Notice` and never costs the members beside it (ADR 0005); `profile` is not a member and does
  not warn.
- 2026-09-04 — Added `crates/confyg-form/src/affordance.rs`: `derive` implements presentation
  §3's precedence (`const`/`readOnly` → display-only, Raw literal fallback → raw text, the
  menu-family thresholds 4/12, `writeOnly` → masked, then the primitive control), `ladder`
  implements presentation §4's table with every chain terminating in a universal control, and
  `resolve` is derive → override → clamp against `HostProfile`. A clamp substitutes but keeps
  `intended` and emits a Notice, so `filterable-menu` legitimately renders as `menu` in v0.1 and
  an unmaskable `writeOnly` value says so rather than silently revealing itself.
- 2026-09-04 — Added `crates/confyg-form/src/compile.rs` and `constraint.rs`: design §4 steps
  1, 2, 4 and 5 — `$ref` resolution with a visited-pointer set cutting cycles into
  `FormNode::Cyclic`, left-to-right `allOf` merging that keeps the narrowest bound and
  diagnoses the rest, classification into Group / Repeat / Field with every v0.1-excluded
  construct (`prefixItems`, `oneOf`, `anyOf`, `additionalProperties` as a schema,
  `patternProperties`) becoming an Unknown node plus a Notice naming its tier, and sibling
  order from the Schema file overridable by `x-confyg.order` with `demoted` sinking and no
  required-hoisting. `SchemaState.validatable` records the `Validator::new` failure so an
  uncompilable `pattern` costs the document its validation, not its form. Snapshots compile the
  checked-in `eslintrc.json` fixture on two Host capability profiles.
- 2026-09-04 — Added `crates/confyg-form/src/overlay.rs` and turned `compile` into
  `project(schema, doc, host)`: design §4 step 6 assigns three-state Presence (an unwritten
  default stays Absent, a node with its own Violations is Invalid and keeps the literal as
  authored), distinguishes `Occupancy::Empty` from `Absent`, marks a YAML alias / merge-key node
  `locked`, projects one Repeat item per Document entry, and attaches Violations by exact Path —
  letting `PointerMap::resolve`'s upward walk put container and `required` failures on the
  container, and an unresolvable pointer on the root. A snapshot asserts that the same logical
  document in TOML, JSON and YAML projects one identical outline (verification item 1).
- 2026-09-04 — Added `crates/confyg-form/src/unknown.rs`: design §4 step 7 sweeps every Document
  key the Schema does not describe into a trailing `FormNode::Unknown` in its own parent, each
  carrying a Notice that names the key — a Notice, never a fabricated Violation, since under
  open `additionalProperties` an extra key breaks no rule (design §7 B18); under
  `additionalProperties: false` the validator's own container Violation stands and the Notice is
  what names the key. `summary` walks the tree depth-first and reports
  `Validation::Unavailable { keyword, pointer }` when the Schema could not compile, so a broken
  `pattern` never reads as "no problems" (C6, D8).
- 2026-09-04 — Added the `confyg-session` crate with `src/ordinal.rs`: `child_ordinal` converts a
  projection index (entries only, the space the Form IR counts in) into the child ordinal a
  `Target` wants, which counts comment nodes too (D7), and subtracts YAML's root comment prefix
  because YAML root indices are container indices. `schema_slot` places a key the Document does
  not hold yet at its Schema `properties` position among present siblings rather than appending
  it blindly, then clamps it before TOML's first capturing `[table]`/`[[aot]]` header, since a
  plain key after one is silently re-keyed into that section (D1). Both bodies carry a
  `// PORTED:` note naming the `pub(crate)` upstream logic they duplicate.
- 2026-09-04 — Added `crates/confyg-session/src/lower.rs`: `SetterIntent::{SetValue, Unset}`
  lower onto `Replace` for a written Field, `Insert` at the Schema slot for an absent one, and
  `Delete` when the value equals the effective default, because the default is written by
  absence (ADR 0003). A Schema-violating value is written and warned about, never refused;
  `Refused` is reserved for an intent the host should not have offered (a required `Unset`, a
  `readOnly` or `locked` Field). Every `Insert` passes `OnCollision::Cancel` and an explicit
  `suggested_key`, and YAML gets a `SetTrailingComment` follow-up because its `Replace` swaps
  the whole entry and would otherwise drop the comment. A key inserted before a documented
  sibling lands above that sibling's leading comment block, not between the comment and the key
  it describes. `tests/roundtrip.rs` runs every case, comment-interleaved included, in all three
  Doc formats (verification item 2).
- 2026-09-04 — Added `crates/confyg-session/src/fragment.rs` and the collection intents
  `AddRepeatItem`, `RemoveRepeatItem` and `ToggleGroup`. Every fragment confyg writes is
  rendered from the Schema in one place, because `Insert` adapts raw text instead of rejecting
  it. `Emission::{Headerless, HeaderBearing}` is the D2 asymmetry: an absent collection takes a
  header-bearing fragment into its parent at the Schema slot, an existing one takes a headerless
  fragment addressed at its own Path so the engine synthesizes the header. Add is gated on
  `maxItems`, Remove on `minItems`, and a template emits only members without a `default`, so
  Minimal write holds for generated text too. `lower` now also takes the Schema, since the IR
  carries a `TemplateRef` pointer rather than inlined text.
- 2026-09-04 — Added `crates/confyg-session/src/template.rs`: `comment_policy` derives the
  Comment policy from the file rather than asking (D4) — TOML and YAML always allow comments,
  a `.json` denies them unless the extension is `.jsonc` or the file arrived already carrying
  them, which is evidence about its consumer. `generate` writes a Template under
  `TemplateStrategy::{RequiredOnly, WithDefaults, Everything}`, turns `title`/`description` into
  leading comments only when the policy allows, and moves the Schema hint into a `"$schema"`
  member when it cannot comment. Each format emits its own hint convention, and the acceptance
  test is C3's reverse direction: `detect_hint` resolves the generated file's own Schema in all
  three formats. This is the only place confyg authors comments; an existing Document's comments
  are never touched.
- 2026-09-04 — Added `crates/confyg-session/src/session.rs`: one `dispatch(Request)` in, one
  `SetterSnapshot` out, and that snapshot is the only type crossing the FFI boundary. The
  session performs no I/O — an unresolved Schema hint leaves a `SchemaFetchRequest` in the
  snapshot for the host, and `Save` only says what the bytes are. With no Schema at all the
  whole Document projects as one `Unknown` node rather than a guessed form (design §6 step 4).
  Undo is a full-text snapshot ring with one entry per *committed* intent, so a refused intent
  costs no step, and each entry carries its Doc format so a format conversion undoes like any
  other edit.
- 2026-09-04 — Added the D9 postcondition guard to `Session::dispatch`: `predicted` states each
  intent's resulting `Shape` from the IR before the write, `observed` reads it off the recompiled
  IR, and a mismatch panics in test builds and becomes a `session.postcondition.mismatch` Notice
  in release ones. Upstream's `Insert` adapts a wrong fragment and reports success, so a write
  that lands in the wrong container is otherwise invisible. `tests/postcondition.rs` runs every
  v0.1 intent and also asserts no generic placeholder key (`__elem__`, `placeholder`) reaches
  the bytes.
- 2026-09-04 — Added `crates/confyg-session/tests/write_neutrality.rs` and a required
  `write-neutrality` CI job: every v0.1 intent runs in three Doc formats against eleven
  `x-confyg` Annotations applied to every property and four Host capability profiles, and every
  run must produce byte-identical output plus an identical refusal log. Verified the property
  bites by making a `Replace` fragment read `unit`: the suite fails naming the Annotation, the
  profile, the format and the case (ADR 0004, verification item 6).
- 2026-09-04 — Added the `confyg-ffi` crate: the WASM boundary is exactly `dispatch(handle,
  request_json) -> snapshot_json` plus `check(handle, path_json, literal)`, and it holds no
  logic, so a native host can link `confyg-session` directly and skip it. A malformed request
  comes back as an error envelope rather than trapping, because a panic across the boundary
  loses the session. `check` runs a live `pattern` check through the *validator's own* engine on
  a throwaway copy of the Document — a lookahead `fancy-regex` accepts and Rust's `regex`
  rejects is asserted, so form warnings and Violations can never disagree. `Request` and
  `SessionCommand` became externally tagged so the FFI JSON has no colliding `kind`.
  `.cargo/config.toml` selects `getrandom`'s JS backend for `wasm32-unknown-unknown`, and CI
  gained a `wasm` job running `wasm-pack build`.
