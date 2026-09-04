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
- 2026-09-04 — Phase A of the v0.1 plan is complete and merged: `confyg-form`, `confyg-session`
  and `confyg-ffi`, 22 test suites, three CI gates. The plan is marked Shipped. Driving the
  session by hand then found two form-level defects the suite does not catch, recorded in
  [`docs/debug/2026-09-04-phase-a-hands-on-findings.md`](docs/debug/2026-09-04-phase-a-hands-on-findings.md):
  a `ToggleGroup` offered where TOML's table-capture rule forbids the insert, and a `Delete`
  that leaves its entry's leading comment block behind.
- 2026-09-04 — `crates/confyg-session/examples/try.rs` with two fixtures: a REPL over
  `Session::dispatch` that renders the form, applies every v0.1 intent and prints the bytes, so
  the write path can be driven before the web host exists.
- 2026-09-04 — Upstream bill item 3, with an `#[ignore]`d regression test: TOML's `Delete` eats
  the blank line after the entry, so a surviving comment block closes up against the next entry
  and silently documents it. Not fixable confyg-side — **Comment policy** forbids deleting the
  user's prose and the closed `Mutation` set has no whitespace operation. YAML and JSON are
  unaffected, and the test's YAML half already passes.
- 2026-09-04 — Phase B, Task 17: the web host's renderer shell. `web/` is an npm workspace on
  Vite and Vitest with `web/src/partition.ts` (presentation §5.1's closed **Partition** set —
  sections are the root's depth-1 **Groups**, and fewer than three of them falls back to
  `scroll`), `web/src/render.ts` (a recursive **Form IR** walk dispatching on `kind`; new code,
  since confy's tree-editor `render.ts` and its `ViewRow` are a value/type row model per
  ADR 0002), `web/src/tokens.css` (upstream's 20 OKLCH chrome roles, `--font`/`--mono` and the
  density tokens inherited as-is; the data-type colours deliberately not ported; `--ghost`,
  `--inherited`, `--violation`, `--notice`, `--locked`, `--required`, `--deprecated`, `--radius`
  added), and `web/src/types.ts` as the transcription of the IR and `SetterSnapshot`.
  `host-io.ts` and `i18n.ts` are ported down from confy to confyg's one host and the Chassis
  catalog `i18n/{en,zh-TW}.json` (`form.*`); `boundary.ts` is the only channel to the core.
  A fourth CI job typechecks, tests and builds the bundle. `--example try` gained a `json`
  command so `web/src/render.test.ts` walks a snapshot the real core produced rather than a
  hand-written IR.
- 2026-09-04 — Phase B, Task 18: the v0.1 widget set. `web/src/widgets/` mounts one control per
  Widget through a `Record<Widget, Mount>` registry, so an unmapped name is a build error rather
  than a blank field — `text` (also `textarea`, `masked`, `rawText`), `menu` (also
  `filterableMenu`, which the Host clamp never lets through), `radio` (also `checkboxSet`),
  `tristate`, `stepper`, `slider` and `display`. Every widget reaches all three **Presence**
  states: a control whose own value space can hold the default renders an `inherited` option,
  one that cannot keeps a separate **Unset** button beside its Ghost text (ADR 0003). A boolean
  is a three-state select, never a bare checkbox (ADR 0002); a **Locked** or `readOnly` field
  mounts as display only whatever its Widget; a clamped Widget names the one it was intended to
  be, from the Chassis lexicon. `render.ts` now delegates the value area to the registry and
  `main.ts` turns a widget's literal into a `setValue` / `unset` intent, whose exact JSON
  `confyg-ffi/tests/boundary.rs` pins against the real binary.
- 2026-09-04 — `FieldMeta` gained `options: Vec<FieldOption>`: the menu family's choices in
  Schema order, each already carrying its label. The core resolved `menu` and `radio` from
  `enum` but never passed the values on, so a host had to re-derive them — and would then honor
  `x-confyg.optionLabels` in one host and not the next.
- 2026-09-04 — Phase B, Task 19: **Repeat** cards, the violation summary, and **Form search**.
  `web/src/repeat.ts` renders a card per object entry and a row per scalar one, titled from the
  core's `labelFrom` through Task 18's own `widgets/common.ts` `editable` — so a card title
  loses the quotes that belong to the encoding, shows an Invalid literal as authored, and
  cannot drift from the field inside the card — and falling back to the item's own title with
  its ordinal; a count badge reads `(3/5)`, or `(3)` where the Schema set no ceiling.
  `+` and `−` are gated by the same two comparisons `confyg-session/src/lower.rs` refuses on,
  so the host never offers an intent the core would decline, and a card publishes its *entry*
  index — the index `RemoveRepeatItem` takes. `web/src/summary.ts` renders design §11's C6
  summary above the form in both **Partitions** and jumps to a node; an uncompilable Schema
  reads *validation unavailable*, never *no problems*, because under D8 a document loses its
  validation and keeps its form.
  `web/src/dom.ts` holds the chrome builders the three new modules share.
- 2026-09-04 — **Form search** landed in the compiler rather than in the host, as presentation
  §5.3 requires: `confyg_form::search::search(&root, query)` fuzzy-matches node titles,
  descriptions and **Paths** with `SkimMatcherV2`, returning hits ranked best-first with ties
  broken on Path so a query never reorders its own results. `fuzzy-matcher` is a direct
  dependency at the version the workspace already resolved, not a route through
  `confy_core::session`, which ADR 0001 gates off. `Session::search` mirrors `check` — it
  recompiles rather than publishing its IR, so no host can re-derive form decisions locally —
  and `confyg-ffi` grew a third export beside `dispatch` and `check`. The host half is only what
  cannot live in the compiler: `web/src/search.ts` maps a hit to the section holding it and
  `render.ts` `reveal` moves the **Partition** there. It shares no code and no term with the
  **Option filter** (§3.1). `--example try` gained `find <query>`, which is how the three
  matching axes were checked on the real binary.
- 2026-09-04 — The `try` fixtures grew the edges the renderer had nothing to render: `servers`
  gained `minItems` and a titled `items` subschema and a third entry, so it sits at its ceiling;
  a scalar `tags` array sits at its floor; and `cacheSize` gained a description whose wording is
  absent from its key. `web/src/__fixtures__/demo-snapshot.json` is regenerated from them, so
  both bounds gates, both item shapes and the non-key search axis are pinned against real core
  output rather than a hand-built IR.
- 2026-09-04 — A bare page load is now a form rather than an empty pane. `web/src/samples.ts`
  imports `confyg-session/examples/demo.{schema.json,toml}` as text and `main.ts` dispatches
  `loadSchema` then `open` at boot, in that order — the Schema is what makes a **Form IR**, and
  an `open` without one renders nothing. Deliberately not a new fixture: it is the pair
  `--example try` drives and `confyg-form/tests/snapshot.rs` pins, so the sample cannot drift
  from what the compiler is tested against, and every teaching edge is already in it — an
  out-of-range `port`, an unknown `colour` kept exactly as written, `tags` at its floor,
  `servers` at its ceiling, an absent optional `tls` **Group**. Modelled on confy's
  `web/samples.ts`, minus its `#:schema` route: confyg's host does not yet act on the
  `SchemaFetchRequest` a hint produces, so the Schema is dispatched as bytes and the gap is
  written down in [`docs/reference/presentation.md`](docs/reference/presentation.md) rather than
  half-implemented. `inSampleMode()` latches while the document has no backing file and drops on
  a real open or a save, so `Save` never offers to write back over a path the user never chose.
  The e2e suite gained the assertion that a bare load renders nodes — which would have caught
  the WASM-glue defect on its own.
- 2026-09-04 — Phase B, Task 20: the **real-binary check**, design §11 item 5. Playwright at the
  repo root (`playwright.config.ts`, `tests/e2e/first-run.spec.ts`) drives the whole flow —
  open → add a **Repeat group** item → set values → **Unset** one → save — against
  schemastore's real `.eslintrc` Schema on the *built* bundle, and asserts the bytes the host
  actually emits: the value written is there, the Unset member is gone, and the untouched region
  is byte-identical. `webServer` builds the bundle and serves it with `vite preview`; the WASM
  build is a prior step, never inside it. CI gains a fifth job, `e2e`, beside `wasm` rather than
  folded into `web`, which stays the sub-minute gate. `savedText` reads a real download:
  deleting `window.showSaveFilePicker` before load takes the non-Chromium branch `host-io.ts`
  already documents, so the download path is the machine-tested one and the in-place File System
  Access write is hand-verified — stated as such in
  [`docs/reference/presentation.md`](docs/reference/presentation.md), like the `#[ignore]`d byte
  test in `crates.md`.
- 2026-09-04 — `docs/reference/` gained one contract file per subsystem — `form-ir.md`,
  `intents.md`, `presentation.md` — describing current behavior, so both design records could
  hand over the v0.1 half of their text and flip to `Superseded in part`. The v0.1 plan moved to
  `docs/plan/README.md`'s Landed table at 20/20. `crates.md` records the fifth gate, the e2e
  test and the command that runs it. Recorded rather than left implicit: **Form search** returns
  a whole subtree at one identical score when a query names a container — the Path is a scored
  axis and every descendant's Path contains the parent key — and v0.1 ships it, because the Path
  tiebreak already orders the rows parent-first and any dominant-ancestor rule that collapsed
  them would also suppress a descendant that matched on its own title or description.
- 2026-09-04 — D1 and D7 reproduced by hand on the real build, both correct: a key inserted into
  a TOML table that already has a sub-table lands inside the table, above the sub-table header,
  in **Schema `properties` order** with the separating blank line intact; a value set in a file
  with a comment between every sibling leaves every comment attached to the entry it documents.
  Recorded in
  [`docs/debug/2026-09-04-real-binary-findings.md`](docs/debug/2026-09-04-real-binary-findings.md).
- 2026-09-05 — Applied the settings-template redesign spec
  ([`docs/spec/20260904-redesign/PATCH.md`](docs/spec/20260904-redesign/PATCH.md)) to `web/` and
  `i18n/`: file chip, dirty dot, violation chip, and a gear app menu (appearance, language,
  density, text size, about/diagnostics, install) replace the old theme button; a Field row is
  now a three-track grid with a `...` menu (`rowmenu.ts`) holding Unset, copy value, raw
  literal, and Schema info; a Repeat that is a section gets an entry column instead of a card
  stack (`partition.ts`'s `SECTIONS_FLOOR` 3 → 2, `Section.repeat`); a coarse pointer or a
  narrow width switches to a pushed single-column shell with a touch back bar. `main.ts` gained
  `shell.init()`, `shell.markDirty()`/`markClean()` around the one write path and the save
  handler, and a `sw.js` registration; `manifest.webmanifest` and `sw.js` were placed under the
  new `web/public/` (not `web/` as the patch table literally states) because `vite build` only
  copies `publicDir` verbatim — confirmed by `dist/` gaining both files only after the move.
  `i18n/{en,zh-TW}.json` merged the two `.additions.json` files; the one overlapping key,
  `form.repeat.add`, took the redesign's text ("Add entry" / "新增一組"). Removed the now-dead
  `#theme` button wiring (`initTheme`/`toggleTheme`) from `main.ts` since `shell.ts` owns theme.
  `partition.test.ts` updated for the new floor. Verified with `tsc --noEmit`, the full
  `vitest` suite (37 tests), a production `vite build`, and both real-binary Playwright e2e
  tests against the built bundle.

### Fixed

- 2026-09-04 — An optional **Group** can now be enabled in TOML. `ordinal::header_slot` clamps a
  header-bearing `Insert` the mirror way to `schema_slot`: upstream accepts a section only at or
  after the parent's first existing header, so `[tls]` at its Schema slot was refused as
  `Illegal("a table here would capture the keys above it")` and the toggle did nothing but emit a
  notice. When the legal floor would split a comment block from the entry it documents, the slot
  steps past that entry instead — legality and comment attachment are hard rules, **Schema
  `properties` order** is not, so it yields. `upstream.md` *Insertion legality* now states the
  real rule. Found by hand, not by the suite; see
  [`docs/debug/2026-09-04-phase-a-hands-on-findings.md`](docs/debug/2026-09-04-phase-a-hands-on-findings.md).
- 2026-09-05 — Phone testing after applying the redesign showed a totally unstyled, overlapping
  page (screenshot-reported by the user as "完全爛掉"). Three bugs, all in the redesign's own
  `web/src/style.css`, none introduced by the port: (1) the file dropped the
  `@import "./tokens.css";` it used to open with, so every design token (`--bg`, `--fg`,
  `--font`, `--accent`, …) was undefined and the browser fell back to its default UA stylesheet
  — Times New Roman, transparent backgrounds, beveled buttons; (2) `#form` was `display: flex`
  with no `flex-direction`, so its two permanent children — the violation summary and the page
  body — sat side by side instead of stacked, running the form off the right edge; (3)
  `body[data-nav="push"] .node.field`'s grid gave the **label** column `minmax(0, 1fr)` (free to
  shrink to nothing) and the **control** column `auto` (sized to its content), the reverse of
  the desktop rule, so a two-word label like "Bind address" or a slider-width control squeezed
  the label into a wrapped, cramped sliver. Fixed by restoring the import, adding
  `flex-direction: column`, and giving the label column a `minmax(90px, 40%)` floor with the
  control column `minmax(0, 1fr)`. Verified against an emulated iPhone 13 viewport before and
  after each fix.
- 2026-09-04 — `FormNode`'s variant fields now serialize as camelCase. `rename_all` renames
  *variants* only, so `raw_preview`, `item_template`, `label_from` and `schema_ptr` crossed the
  boundary in snake_case against the IR's documented contract; `rename_all_fields` fixes it.
  Found by feeding a real snapshot to the new renderer, where `Unknown` rendered an empty
  preview.
- 2026-09-04 — The built web host now loads the core. `boundary.ts` imports the `wasm-pack` glue
  through a non-literal specifier so a fresh checkout typechecks before the WASM exists, which
  left the browser resolving `/crates/confyg-ffi/pkg/confyg_ffi.js` — a URL neither the dev
  server nor `dist/` served, so every page load ended in `Failed to fetch dynamically imported
  module` and an empty form. A `web/vite.config.ts` plugin makes that one URL real in both,
  serving the generated directory in dev and copying it beside the bundle on build, skipped when
  the WASM is absent so CI's `web` job still builds. The glue is copied, never bundled, and
  `boundary.ts` is unchanged. The renderer had never talked to the core in a browser; no unit
  test could see it, because the jsdom suite renders a captured `SetterSnapshot`.
- 2026-09-04 — An **Absent** Repeat can now be given its first item. `style.css` hid
  `.repeat-add` at `data-occupancy="absent"`, which is exactly the collection **Absent-parent
  lowering** exists to fill — `lower.rs` puts a header-bearing fragment in the parent for that
  case and design §11 item 2 makes it mandatory in every format — so every absent array in every
  Schema was unfillable from the web host. The rule no longer names the button; an absent Repeat
  still collapses its empty `.repeat-items` and keeps its heading and count.
- 2026-09-04 — The `rust` CI gate reads code again. `rg -q 'confy_core::session' crates/` matched
  the prose in `confyg-form/src/lib.rs`'s own module docs — which say the module is never
  referenced — so the ADR 0001 gate had been failing on a sentence rather than on a reference.
  The pattern now skips comment lines and still catches both a `use` and an inline path.

### Known issues

- 2026-09-04 — A text edit committed on blur swallows the next click: the commit re-renders the
  form by replacing `#form`'s children, so a control clicked immediately after typing is
  detached mid-click and its intent is lost, once, silently. Repro, evidence that the core and
  the boundary are not involved, and why the fix is a renderer decision rather than a patch are
  in
  [`docs/debug/2026-09-04-real-binary-findings.md`](docs/debug/2026-09-04-real-binary-findings.md)
  finding 3. `tests/e2e/first-run.spec.ts` sequences around it with an explicit `blur()` and says
  so at the call site.
- 2026-09-05 — The redesign's own goal — "a row is label | control | one 30px button"
  (`docs/spec/20260904-redesign/PATCH.md`) — is not fully realized: `PATCH.md` never lists
  `web/src/widgets/{common,text,stepper,slider,radio}.ts` for change, so `unsetButton` (ADR
  0003) still renders its own `.unset` button inside `.control` on every widget that had one,
  alongside the new `...` menu's own Unset item — two ways to unset the same field rather than
  one. Left as found rather than removed unilaterally, since retiring `unsetButton` is a design
  decision the patch document doesn't make.
- 2026-09-05 — `manifest.webmanifest`'s three `/icons/icon-*.png` are not in the repo (no
  `web/public/icons/`); installing confyg will show a broken/generic icon until real assets are
  added.
- 2026-09-05 — The violation chip (`#issues`) can never appear: `shell.ts` shows it only when
  `#form .summary` is hidden, but nothing in the patch ever sets `summary.hidden = true` — there
  is no dismiss control on the summary itself. The chip's `click` handler (un-hide the summary)
  is reachable code with no way to reach it.
- 2026-09-05 — `PATCH.md`'s `main.ts` wiring section instructs calling `shell.onSnapshot(snapshot)`
  after `render(...)`, but `Shell` defines no such method — `render.ts` already calls
  `appShell.afterRender()` itself at the end of `render()`. Skipped as redundant/nonexistent
  rather than wired, since adding it would not compile.

