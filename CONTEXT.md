# CONTEXT

Entry point for all documentation. Root-level files (`README.md`, `CHANGELOG.md`, `LICENSE`,
`PRIVACY.md`) stay here; everything else lives under `docs/`.

| Folder | Holds | Canonical? | Lifecycle |
|---|---|---|---|
| [`docs/reference/`](docs/reference/README.md) | Current behavior: glossary, per-subsystem contracts | Yes — the only source of truth | Kept in sync with the code |
| [`docs/adr/`](docs/adr/README.md) | Decisions that were expensive to reach and would be expensive to reverse | No — historical | Never edited; superseded by a new ADR |
| [`docs/spec/`](docs/spec/README.md) | Design records written before implementation | No — historical | Frozen once approved; only `Status:` changes |
| [`docs/plan/`](docs/plan/README.md) | Task-by-task implementation plans derived from a spec | No — historical | Frozen once shipped; only `Status:` changes |
| [`docs/debug/`](docs/debug/README.md) | Handoff notes from investigations, with repro scripts | No — historical | Frozen once resolved; only `Status:` changes |
| [`docs/audit/`](docs/audit/README.md) | Point-in-time sweeps for bugs, dead code, inconsistency | No — historical | Frozen once findings are addressed; only `Status:` changes |
| `docs/tmp/` | Scratch | No | Archived to `tmp/archive/YYYY-MM.tar.gz` when stale |

## Reading order

1. [`docs/reference/glossary.md`](docs/reference/glossary.md) — the vocabulary every other file uses.
2. [`docs/reference/upstream.md`](docs/reference/upstream.md) — what `confy-core` actually
   provides, and the two changes confyg needs from it.
3. [`docs/reference/README.md`](docs/reference/README.md) — the subsystem map.
4. [`docs/adr/README.md`](docs/adr/README.md) — why the shape is what it is.
5. [`docs/spec/README.md`](docs/spec/README.md) — the design records.
6. `CHANGELOG.md` — what changed recently.

## Upstream

confyg depends on [confy](https://github.com/superyngo/confy)'s `confy-core` crate for the
lossless CST **Document** model and the JSON Schema subsystem. confy's
`docs/reference/CONTEXT.md` is the source of truth for every glossary term marked
*(inherited)*; confyg never restates or forks those definitions.
