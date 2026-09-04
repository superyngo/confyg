# Phase A hands-on findings

Status: Open

Two defects found by driving the real session by hand after Phase A landed (16/16 tasks, suite
green). Neither is a test failure — both are cases where the shipped behavior is defensible but
the *form* is wrong about it. Repro is a throwaway REPL, not a product surface.

## Repro

```
cargo run -p confyg-session --example try -- <schema.json> <config.toml>
```

`crates/confyg-session/examples/try.rs` is uncommitted scratch; recreate it as a REPL over
`Session::dispatch` if it is gone. Fixtures used:
`docs/tmp/claude-scratch/demo.schema.json` and `demo.toml` (also scratch, also uncommitted) — a
schema with an optional `tls` object ordered *before* `servers`, and a TOML document whose root
holds a plain `colour` key after the schema's own keys.

## 1. The form offers a `ToggleGroup` TOML cannot honor

`on tls` returns `Notice [session.mutate.failed] Illegal("a table here would capture the keys
above it")` and writes nothing.

Correct refusal: **Schema `properties` order** puts `tls` before `servers`, so `[tls]` lands
above the plain `colour` key and would capture it. The bug is that the **Form node** carried a
`GroupToggle` at all — the host rendered a control whose only outcome is a notice. `Refused`
exists precisely to make this nameable, and it was not used.

Two candidate fixes, and they are not equivalent:
- Clamp the section insert to a legal partition (`ordinal::clamp_to_partition` already computes
  the boundary), writing `[tls]` after the last plain root key instead of at its Schema slot.
  Keeps the affordance, breaks **Schema order** for that one node.
- Suppress the toggle when the target position is illegal in the document's **Doc format**.
  Keeps ordering, removes a control the user can see in JSON/YAML but not TOML — a
  format-dependent form, which cuts against verification item 1.

The first is probably right (legality already wins over Schema order at the TOML root — see
`ordinal.rs`), but it is a design decision, not a patch.

## 2. `Delete` orphans the entry's leading comment block

`set port 8080` where `8080` is the effective `default` lowers to `Delete` per ADR 0003
(**Minimal write**). The entry goes; its leading comment does not:

```toml
# deliberately out of range, so you can see an Invalid field
# a key the schema has never heard of
colour = "puce"
```

The first comment now documents `colour`. This is the mirror of the hazard Task 9 fixed on the
insert side (a new key lands *above* a sibling's leading comment block, never between the
comment and its key) — the delete side was never given the same rule.

Whether the comment should be deleted with the entry or left as an orphan is a policy question,
and it interacts with `Mutation::Remark` (v0.2): a commented-out entry *is* a comment block, so
"delete the leading comments" must not eat a remarked sibling. Check whether `confy-core`'s
`Delete` has a comment-carrying variant before adding one confyg-side.
