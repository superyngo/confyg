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
