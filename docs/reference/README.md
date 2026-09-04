# Reference

Current behavior only. Anything historical — a superseded design, a shipped plan, a resolved
investigation — lives in [`../spec/`](../spec/README.md), [`../plan/`](../plan/README.md),
[`../debug/`](../debug/README.md), or [`../audit/`](../audit/README.md), not here.

- **[glossary.md](glossary.md)** — canonical vocabulary; read first.
- **[upstream.md](upstream.md)** — `confy-core`'s pin, reachable API, index spaces, and the
  upstream changes confyg requires. Canonical for every fact about the dependency.
- **[crates.md](crates.md)** — what each crate owns, the entry points a host calls, and the CI
  gates.

Machine-checked: `cargo test --workspace`.

Terms marked *(inherited)* in the glossary are owned by
[confy](https://github.com/superyngo/confy)'s `docs/reference/CONTEXT.md`.

See also [`../adr/`](../adr/README.md) for decision records.
