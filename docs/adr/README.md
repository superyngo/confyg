# Architecture Decision Records

One file per decision that was expensive to reach and would be expensive to reverse. An ADR
records *why* and which alternatives were rejected; it is a historical record, never edited.
Current behavior lives in [`../reference/`](../reference/README.md).

| # | Decision | Status |
|---|---|---|
| [0001](0001-confy-core-as-pinned-git-dependency.md) | `confy-core` is a pinned git dependency with `session` feature-gated off, not a fork, monorepo, or published crate | Proposed |
| [0002](0002-schema-driven-projection-and-three-state-presence.md) | The Schema drives projection and the Document is an overlay; every Field carries three-state Presence | Proposed |
| [0003](0003-form-ir-shape-presence-scope-and-default-equivalence.md) | Presence stays a Field concept while containers carry Occupancy; a scalar array is a Repeat group; setting a value equal to the default is Unset | Proposed |
| [0004](0004-presentation-layer-model-and-write-neutrality.md) | Eight presentation layers with Emission style and Conduct closed; Write-neutrality is an invariant; Flow never gates on validity; host capability clamps at compile time | Proposed |
| [0005](0005-presentation-profile-as-a-second-carrier.md) | One Presentation vocabulary with two carriers — an `x-confyg` Annotation and an optional Presentation profile sidecar; supersedes the glossary's rejection of a separate UI-schema file | Proposed |

Volatile facts an ADR happens to quote — a dependency pin, a line count, an upstream API shape —
are corrected in [`../reference/upstream.md`](../reference/upstream.md), not by editing the ADR.
An ADR may carry a dated erratum line pointing there; its decision and rationale stay immutable.
