# Phase A hands-on findings

Status: Resolved — finding 1 fixed, finding 2 deferred to upstream bill item 3

Two defects found by driving the real session by hand after Phase A landed (16/16 tasks, suite
green). Neither is a test failure — both are cases where the shipped behavior is defensible but
the *form* is wrong about it. Repro is a throwaway REPL, not a product surface.

## Repro

```
cargo run -p confyg-session --example try -- <schema.json> <config.toml>
```

The REPL is `crates/confyg-session/examples/try.rs`, and the fixtures it names live beside it:

```
cargo run -p confyg-session --example try -- \
  crates/confyg-session/examples/demo.schema.json crates/confyg-session/examples/demo.toml
```

The schema orders an optional `tls` object *before* `servers`; the document's root holds a plain
`colour` key after the schema's own keys, and comments above entries throughout.

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

### Resolution: the first, with one correction

`ordinal::header_slot` now clamps a header-bearing insert the mirror way, and `lower::grow` uses
it. The correction: upstream's rule is not "past the last plain key" but `index >= split`, the
parent's *first* capturing-header child (`check_partition`), so a section can never precede an
existing one — appending is the only legal spot in a document of plain keys alone. That floor
can also land between a comment block and the entry it documents, so the clamp steps past that
entry: legality and comment attachment are hard rules, Schema order is not, and it yields. Both
facts are now in `upstream.md` *Insertion legality*.

Tests: `ordinal.rs` `a_section_never_lands_before_a_plain_key` and
`only_toml_clamps_a_section_at_all` (the form must not become format-dependent);
`roundtrip.rs` `enabling_a_section_never_captures_the_keys_above_it`, three formats.
Verified on the real binary: `on tls` writes `[tls]` and reads back `{populated}`, with every
comment still on its own key.

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

Whether the comment should be deleted with the entry or left as an orphan turned out not to be
the question. Three probes pin the real rule:

| Source | After the delete |
|---|---|
| `port = 1` / blank / `host = "x"` | `host = "x"` |
| `# lead` / `port = 1` / `host = "x"` | `# lead` / `host = "x"` |
| `host = "x"` / blank / `port = 1` | `host = "x"` / blank |

The comment is not the casualty — the *blank line* is. TOML's `detach_entry_line` detaches the
entry's following `NEWLINE` token, and taplo lexes a run of newlines as one token, so the
separator goes with it and the surviving block closes up against the next entry. YAML and JSON
are unaffected.

### Resolution: deferred, because confyg cannot reach it

Deleting the comment with the entry is ruled out by **Comment policy** — an existing Document's
comments are never touched, and losing the user's prose is worse than losing a blank line. The
closed `Mutation` set has no whitespace operation, so the fix belongs upstream: recorded as
**upstream bill item 3** with this repro. `roundtrip.rs`
`a_delete_keeps_the_blank_line_that_separates_a_comment_block` asserts the wanted bytes in TOML
*and* YAML and is `#[ignore]`d; the YAML half already passes, so removing the attribute the day
the pin moves is the whole verification.
