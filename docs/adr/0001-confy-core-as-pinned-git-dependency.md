# 0001 — confy-core as a pinned git dependency behind a `session` feature flag

Date: 2026-09-03

> **Erratum, 2026-09-04.** Two facts quoted below are wrong and are corrected in
> [`../reference/upstream.md`](../reference/upstream.md), which is canonical for them: the tag
> `v1.0.1` was never pushed (confy's newest tag is `v1.0.0`, and no released tag carries the
> `session` feature this decision requires), and `confy-core/model/**` is ~21,500 lines across
> all three backends rather than ~11,000. Neither changes the decision — the second strengthens
> it. The dependency line below is superseded by that file.

## Decision

confyg is a separate repository that depends on
[confy](https://github.com/superyngo/confy)'s `confy-core` crate as a git dependency pinned to a
release tag, with `default-features = false` so only the `model` and `schema` modules compile in:

```toml
confy-core = { git = "https://github.com/superyngo/confy", tag = "v1.0.1", default-features = false }
```

This requires one non-breaking change in confy: a `session` Cargo feature, on by default, gating
`pub mod session;`. confy's own hosts keep the default feature set and are unaffected.

## Why

The reusable asset is large and expensive to reproduce. `confy-core/model/**` is roughly 11,000
lines of lossless rowan CST work — a taplo-backed TOML backend plus hand-rolled JSON(C) and YAML
parsers, projections, and value-replacement engines — and `confy-core/schema/**` is another ~950
lines covering JSON Schema 2020-12 validation, per-format **Schema hint** detection, RFC 6901
pointer-to-**Path** mapping, and the **Soft constraint** model. Duplicating that would double the
maintenance surface of three parsers for no gain, and the two products would drift apart on
exactly the behavior a user notices: round-trip fidelity.

The part confyg does *not* want is equally large and cleanly separated: `confy-core/session/**`
is ~8,700 lines of editor machinery (clipboard, multi-row selection, arbitrary add-node picker,
notation switching, type-facet filter, character-level inline edit buffer). A feature flag drops
it at compile time instead of asking confyg to depend on and ignore it.

## Alternatives rejected

**A monorepo — confyg as new crates inside confy's workspace.** Refactoring is easiest and
`model` changes propagate instantly. Rejected because the two products would then share one
release pipeline, one `CHANGELOG.md`, one `RELEASES.md`, and one tag namespace, while shipping to
different audiences with different version cadences. confy's repository identity — "a TUI for
editing config files" — would be diluted into a two-product monorepo, and every confyg release
would drag confy's build matrix with it.

**Extracting a `confy-doc` crate published to crates.io, depended on by both.** The cleanest
boundary and the only option that makes the document model reusable by third parties. Rejected
as premature: it demands semantic-versioned API stability today, while both the `model` surface
and confyg's needs are still moving. Paying that cost before the **Form IR** has stabilized would
force either version churn or a frozen-too-early API. This remains the intended end state; see
Consequences.

**Vendoring a trimmed copy of `model` and `schema` into confyg.** Rejected outright: three
parsers maintained twice, with round-trip bugs fixed in one copy and not the other.

## Consequences

- confyg is coupled to a confy tag. A `model` or `schema` change in confy reaches confyg only
  when the pin moves, which is deliberate: confyg upgrades on purpose, not by surprise.
- confyg cannot change `confy-core`. Anything confyg needs from the document or schema layer is a
  pull request against confy, kept generic enough to serve both products, or it lives in confyg's
  own crates on top.
- The `session` feature is a compatibility surface. Removing it, or moving a type confyg uses
  behind it, breaks confyg's build.
- When the `model` API and the **Form IR** have both settled, this decision should be revisited in
  favour of the extracted-crate option. That will be a new ADR superseding this one.
