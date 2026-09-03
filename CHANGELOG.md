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
