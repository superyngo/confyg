# Intents

Everything a host can ask a session to write, and what reaches the file. Canonical for the
*contract*; design §5 and §8 hold the reasoning.

One call does all of it:

```rust
confyg_session::session::Session::dispatch(Request) -> SetterSnapshot
```

`Request` is externally tagged — `{"intent": {...}}` or `{"command": {...}}` — because the FFI
boundary carries it as JSON and an internal tag would collide with a command's own `kind`.
`confyg_ffi::dispatch` is the same call, JSON in, JSON out.

## The five Setter intents

Internally tagged on `kind`, camelCase. `confyg-ffi/tests/boundary.rs` pins the exact JSON
`web/` builds.

| `kind` | Fields | Writes |
|---|---|---|
| `setValue` | `path`, `value` | The literal, or `Delete` when it equals the effective default |
| `unset` | `path` | `Delete` — never an empty value |
| `addRepeatItem` | `path` | One entry, through one of two lowerings |
| `removeRepeatItem` | `path`, `index` | `Delete` at an **entry** index |
| `toggleGroup` | `path`, `enable` | The Group's template, or a `Delete` of the whole section |

`index` is an entry index. The core converts to the Document's child index itself
(`confyg_session::ordinal`), so a comment is never counted — see [`upstream.md`](upstream.md)
*Index spaces*.

The seven remaining intents in design §5 — `Remark` among them — are v0.2 and later. The enum is
extended, never renamed.

## The two rules every arm obeys

**Minimal write.** A value equal to the effective `default` lowers to `Delete`, never to a
`Replace` that writes the default out (ADR 0003). Setting a field to its default is therefore
**Unset**, and the round-trip matrix asserts it in all three formats.

**Soft constraint.** A value that violates the Schema is *written* and warned about, never
refused. Validation informs; it does not gate.

## Refusal is a bug signal, not a user path

`Refused` is returned only for an intent the host should not have offered: no node at that Path,
the wrong node kind, `Add` at `maxItems`, `Remove` at `minItems`, `Unset` on a required or
**Locked** Field. Every one of those is a host-side predicate the host has the data to check
first, which is why `web/src/repeat.ts`'s two bounds gates are deliberately the same two
comparisons `lower.rs` refuses on.

A user typing something invalid never produces a `Refused`.

## Absent-parent lowering

`addRepeatItem` and `toggleGroup` choose their lowering on **Occupancy** — the D2 asymmetry:

- **Absent** collection: a *header-bearing* fragment inserted into the **parent**, at a slot
  clamped past the plain keys a section would otherwise capture (`ordinal::header_slot`).
- **Existing** collection: a *headerless* fragment addressed at the collection's own Path, at
  `child_ordinal(doc, path, len)`.

Both carry `suggested_key` and `OnCollision::Cancel`. "First item of an absent collection" versus
"next item of an existing one" is a `Target` choice, never a difference in fragment text.

This is the path that had no reachable affordance in the web host until the real-binary check
found it ([`../debug/2026-09-04-real-binary-findings.md`](../debug/2026-09-04-real-binary-findings.md)
finding 2).

## Fragments and their placeholders

`confyg_session::fragment` is the one place confyg *authors* text. A member with an effective
`default` is written by absence, so a fragment never emits it; everything else gets the empty
literal of its declared type — `0`, `false`, `[]`, `{}`, or `""`.

The type is read off the member's own `type` keyword. **A member with no `type` — a `$ref` or a
`oneOf` — gets `""`**, including when the resolved subschema is an object or an array. On
schemastore's `.eslintrc`, adding an `overrides` entry therefore writes `"ecmaFeatures": ""`,
`"rules": ""` and `"env": ""` where the Schema wants objects. The entry is still valid text and
still recompiles, and the members it mistypes are the same `$ref`/`oneOf` ones v0.1 projects as
`unknown` anyway (see [`form-ir.md`](form-ir.md) *What v0.1 does not project*), so resolving them
is the same v0.2/v0.3 work rather than a separate fix.

Comments are emitted here and nowhere else. An existing Document's comments are never touched,
and **Comment policy** is derived from the file, never chosen: strict `.json` gets none, a
`.jsonc` or a `.json` that arrived carrying comments does.

## The postcondition guard

An intent counts as applied only when the mutated Document *recompiles to the Form IR shape the
intent predicted*. That is the D9 guard, it is cheap because the pipeline recompiles anyway, and
`confyg-session/tests/postcondition.rs` runs it for every intent. A fragment that wrote a
different structure and reported success cannot pass it.

## Commands

| Command | Does |
|---|---|
| `open { text, fmt, path }` | Parses, clears the undo ring, compiles |
| `save` | **Nothing.** Saving is the host's I/O; every snapshot already carries the bytes |
| `convertFormat(fmt)` | Rewrites the document in another **Doc format** |
| `loadSchema { source, text }` | Installs a Schema the host fetched, clears the pending fetch |
| `undo` / `redo` | Steps the full-text snapshot ring |

Undo is a full-text snapshot ring, one entry per committed intent: with a lossless CST the whole
text is the cheapest correct undo unit, and it cannot drift from the tree the way a replayed
mutation log can.

`save` doing nothing is load-bearing — it is what keeps the session free of I/O in a browser, a
terminal and an extension host alike.

## What comes back

`SetterSnapshot` is the only type that crosses the FFI boundary, and hosts consume exactly these
fields:

| Field | Is |
|---|---|
| `ir` | The **Form IR** root |
| `summary` | Every attributed Violation, plus whether validation could run |
| `text` | The document as it now stands — the bytes a host saves |
| `notices` | What the compiler or a mutation had to say |
| `fetch` | A Schema the host must resolve, because the session will not |
| `canUndo` / `canRedo` | Ring state |

A malformed request comes back in the same envelope as `{ "error": ... }` rather than trapping,
so a host shows the message instead of losing the session.

Two more calls exist for the live cases, both recompiling rather than publishing the IR — a
public accessor would let a host read the compiled tree and start deciding things locally:

- `Session::check(path, literal) -> Vec<Violation>` — an uncommitted buffer, through the
  *validator's* engine, so a form warning and a Violation cannot disagree.
- `Session::search(query) -> Vec<Hit>` — **Form search** against the document as it stands
  ([`presentation.md`](presentation.md) *Form search*).
