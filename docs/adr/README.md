# Architecture Decision Records

One file per decision that was expensive to reach and would be expensive to reverse. An ADR
records *why* and which alternatives were rejected; it is a historical record, never edited.
Current behavior lives in [`../reference/`](../reference/README.md).

| # | Decision | Status |
|---|---|---|
| [0001](0001-confy-core-as-pinned-git-dependency.md) | `confy-core` is a pinned git dependency with `session` feature-gated off, not a fork, monorepo, or published crate | Proposed |
| [0002](0002-schema-driven-projection-and-three-state-presence.md) | The Schema drives projection and the Document is an overlay; every Field carries three-state Presence | Proposed |
