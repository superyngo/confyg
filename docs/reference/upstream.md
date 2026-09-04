# Upstream: confy-core

Current facts about confyg's dependency on [confy](https://github.com/superyngo/confy)'s
`confy-core` crate. This file is canonical and kept in sync with the code; the *decision* to
depend on confy this way lives in [ADR 0001](../adr/0001-confy-core-as-pinned-git-dependency.md)
and is not restated here. Verified against confy `main` on 2026-09-04.

Volatile facts belong here, not in an ADR or a spec, so that a correction never requires
editing a frozen document.

## The pin

No released confy tag supports confyg, and `confy-core` declares **no Cargo features at all** on
`main`: `pub mod session;` is unconditional, so `default-features = false` is currently a no-op
rather than a gate. confyg passes it anyway — it is forward-compatible with the `session` feature
requested in *The upstream bill*, and until that lands the CI grep for `confy_core::session` is
what actually enforces ADR 0001. Until a supporting tag exists, confyg pins a revision.

```toml
# during development
confy-core = { git = "https://github.com/superyngo/confy", rev = "558bee7fd7317914662e5133b8a47aa7803bbb5b", default-features = false }
serde_json = { version = "1", features = ["preserve_order"] }
```

That revision is confy `main` as of 2026-09-04. At confyg v0.1 this becomes `tag = "v1.1.0"` — a
*minor* bump, because a new Cargo feature and a new public helper are additive API, not a patch.
`Cargo.lock` is committed so a moved branch cannot silently change the build.

Note for the record: `tag = "v1.0.1"` does not exist. confy's `Cargo.toml` says `1.0.1` and its
`CHANGELOG.md` has a `[v1.0.1]` section, but the tag was never pushed; `v1.0.0` is the newest.

## Reachable API

Everything confyg needs from the document and schema layers is `pub`. The re-export surface is
thin: items live in their defining modules, so the imports are
`confy_core::model::any_doc::AnyDocument` and `confy_core::model::document::{ConfigDocument,
DocFormat}` — `model::AnyDocument` does not resolve. `ConfigDocument` must be imported as a trait
for `apply` / `serialize` to be callable.

| Need | Item |
|---|---|
| Parse | `AnyDocument::from_str_as(text, DocFormat) -> Result<AnyDocument, ParseError>` |
| Mutate | `ConfigDocument::apply(&mut self, Mutation) -> Result<String, MutateError>` — commits and returns the new serialized text |
| Serialize | `ConfigDocument::serialize(&self) -> String` |
| Address | `Path = Vec<Seg>`, `Seg::{Key, Index}`; `Target { pub parent: Path, pub index: usize }` |
| Collide | `OnCollision::{Overwrite, Rename, Cancel}` |
| Validate | `schema::validate(&serde_json::Value, &jsonschema::Validator, &PointerMap) -> Vec<Violation>` — no `Session` needed |
| Map pointers | `PointerMap::resolve(&str) -> Option<&Path>`, built by `schema::value_bridge::bridge` |
| Report | `Violation { path, pointer, keyword, message, category }`, `Category::{Value, Representation}` |
| Schema hint | `schema::hints::detect_hint(text, DocFormat) -> Option<SchemaSource>` |
| Convert format | `model::convert::convert(&AnyDocument, DocFormat) -> Result<ConvertResult, ConvertAbort>` |
| Projection tree | `Node`, `NodeTree`, `NodeKind`, `Format`, `next_available_key` |

`Mutation` has **ten** variants, not five: `Delete`, `Insert`, `Replace`, `Rename`, `Remark`,
`EditComment`, `InsertComment`, `Move`, `ConvertKind`, `SetTrailingComment`. `Insert` carries
`on_collision: OnCollision` and `suggested_key: Option<String>`; `Move` takes
`sources: Vec<Path>`. confyg uses `Replace`, `Insert`, `Delete`, `Rename`, and `Move` (v0.2),
plus the comment mutations for Template generation only. `ConvertKind` is notation switching and
is never used.

## Not reachable

These are `pub(crate)`. confyg works around each without an upstream change:

| Item | Location | confyg's answer |
|---|---|---|
| `aot_group_*` | `model::cst_edit::aot_group` | Not needed: `Insert` with a headerless fragment addressed at the array path synthesizes the `[[…]]` header |
| `check_partition` | `model::cst_edit::move_paste` | confyg computes the root partition itself from the public `Node` / `NodeKind` / `Format` |
| numeric clamp / step | `session::schema_hint` | Owned by `confyg-form`. The module is misnamed — it is numeric nudging, not schema hints |
| `undo_redo` | `session::undo_redo` | Ported; 69 lines of `impl Session` |
| schema-fetch protocol | `session::{dispatch, intent, session}` | `confyg-session` defines its own request/response, reusing `SchemaSource` |
| `true_sibling_index` | `session::insertion` | Ported for v0.1; requested upstream as `insert_ordinal` (see below) |

## The upstream bill

Three structural changes confyg cannot work around, all PRs against confy:

1. **A `session` Cargo feature**, on by default, gating `pub mod session;`. Verified safe: neither
   `model/` nor `schema/` references `session` in executable code (zero matches for
   `crate::session`), so gating compiles cleanly.
2. **A public insertion-ordinal helper** in `model::` — the projection-index → child-ordinal
   mapping currently trapped in `session::insertion::true_sibling_index`. See *Index spaces*.
3. **`Delete` must not eat the blank line after the entry.** TOML's `detach_entry_line` detaches
   the entry's following `NEWLINE` *token*, and taplo lexes a run of newlines as one token, so
   deleting an entry that is followed by a blank line closes the gap:
   `port = 1\n\nhost = "x"\n` becomes `host = "x"\n`. When the deleted entry carried a leading
   comment block, that block then reads as the *next* entry's documentation — the same
   misattribution *Index spaces* makes confyg guard on the insert side, arriving from the delete
   side instead. Deleting the comment with the entry is not the fix: **Comment policy** says an
   existing Document's comments are never touched, and losing the user's prose is worse than
   losing a blank line. JSON and YAML are unaffected. Until it lands, confyg's minimal-write
   `Delete` (ADR 0003) can silently re-attribute one comment block; `roundtrip.rs`
   `a_delete_keeps_the_blank_line_that_separates_a_comment_block` is `#[ignore]`d against it.

Deliberately kept in confyg instead of upstreamed (pure computation, confyg's cadence):

- Schema keyword introspection for `default`, `examples`, `required`, `deprecated`, `readOnly`,
  `writeOnly`, `prefixItems`, `additionalProperties`. `schema::hints_edit` reads none of these —
  only `title` (for `oneOf` labels) and `additionalProperties`. Offer upstream later if confy's
  hover wants it.
- Numeric clamping and stepping.

## Index spaces

`Target.index` is **not** the index space a `Path` uses. This is the highest-severity trap in the
mutation API: getting it wrong misplaces text rather than raising an error.

- `Seg::Index(n)` in a `Path` is a **projection** index — comments are not counted.
- `Target.index` is an ordinal in the parent's **full child sequence, comments included**. YAML
  additionally subtracts a `root_prefix_offset`.
- For `Mutation::Move`, `target.index` is a **pre-deletion** ordinal; the engine then subtracts
  the number of same-container sources sitting below it.
- A comment **block**, not a comment line, is one child: consecutive `#` (or `//`) lines merge
  into a single `Comment` node in all three backends, and a blank line splits them
  (`model/cst_project.rs`). Three comment lines above the first entry therefore shift a
  `Target.index` by one, not by three.

confyg converts between the two before every `Insert`. No upstream helper is public today; see
the upstream bill.

## Insertion legality

TOML requires a table's scalar members to precede its sub-tables. The engine's behaviour is
**asymmetric**:

- Targeting a **sub-table** clamps silently — an entry lands no later than the partition split, a
  header no earlier.
- Targeting the **document root** does not clamp; an out-of-partition index returns
  `MutateError::Illegal`.

The root rule for a header-like fragment is coarser than "captures nothing": `check_partition`
accepts it only at an index `>= split`, where `split` is the parent's *first* capturing-header
child (a dotted-key table is not one), or the child count when there is none. A section can
therefore never be inserted above an existing section, whatever **Schema `properties` order**
says, and a document of nothing but plain keys can only take one at the end.

That floor can land exactly between a comment block and the entry it documents, which is the
misattribution *Index spaces* forbids. Both rules are hard, so confyg resolves the collision by
stepping past the documented entry and letting Schema order — the one soft rule of the three —
yield. `ordinal::header_slot` is that computation; `ordinal::schema_slot` is the plain-key
mirror.

So confyg computes the root-level slot itself. There is no upstream helper that answers "where
among these siblings does key K belong" — schema-ordered placement is entirely confyg's.

## Fragment contract

`Insert` takes raw text and *adapts* it rather than rejecting it, so a wrong shape can write a
structurally different document and report success:

- A bare value into an object/table/mapping is keyed with `suggested_key`, or with a generic
  `placeholder` when that is `None`.
- A keyed entry into an array is wrapped into an inline table / single-member object.
- A section header into an array-of-tables is rejected: `MutateError::Illegal`.

confyg therefore always passes a fully-formed fragment **and** an explicit `suggested_key`, so
`placeholder` can never appear in a confyg-written file. Fragments are reparsed and the whole
document is re-validated after each mutation.

## Schema validation

- Validator: `jsonschema` 0.30, `default-features = false`. Drafts 4 / 6 / 7 / 2019-09 / 2020-12
  are auto-detected from `$schema`; **absent `$schema` defaults to 2020-12**. confy never sets a
  draft explicitly.
- `pattern` is compiled with **`fancy-regex`**, so ECMA-262 lookaround and backreferences are
  accepted. confyg's live checks must use this same engine — Rust `regex` would reject patterns
  the validator accepts, and form warnings must never disagree with **Violations**.
- An uncompilable `pattern` makes `Validator::new` return `Err`, so **one bad regex costs the
  whole document its validation**. confyg models that as a document-level state, not as silence.
- Container keywords report the **container's** path: `minItems`, `maxItems`, `uniqueItems`,
  `required`, `minProperties`, `additionalProperties`.
- `PointerMap::resolve` walks *up* the pointer until it finds a mapping, so a violation at an
  absent path attributes to the nearest present ancestor; an unresolvable pointer falls back to
  the root.
- `additionalProperties: false` names the offending keys in the message only — an unknown key
  cannot be attributed to its own **Path** from the `Violation` alone.

## `serde_json` `preserve_order`

Cargo feature unification makes confyg's choice global to the build graph, so it applies to
`confy-core` too. Verified safe: no `serde_json::Map` touches document parsing, mutation,
serialization, or format conversion — those use Rowan CSTs and an ordered
`Value::Map(Vec<Item>)`. It reaches only `schema::value_bridge` and `schema::hints_edit`, and no
fixture asserts alphabetical ordering. A one-line PR adding it upstream is optional politeness so
confy's CI covers the configuration confyg ships.

## Formats and comments

`DocFormat` is `{ Toml, Json, Yaml }`. There is **no** JSONC variant — `detect_format` maps both
`.json` and `.jsonc` to `Json`, and `JsonDocument` accepts comment mutations unconditionally
(`had_comments_at_open` is a load-time advisory, never a write gate). Enforcing "strict JSON has
no comments" is entirely confyg's job; see **Comment policy** in the glossary.

`Mutation::Remark` is an idempotent toggle in all three formats: it comments a live entry out, and
uncomments an existing comment back into a live node by reparsing.

## Tests to imitate

Exact-bytes round-trip patterns confyg's own tests should copy:
`crates/confy-core/tests/roundtrip.rs`, `roundtrip_json.rs`, `roundtrip_yaml.rs`, plus the
in-module suites `model/cst_edit/tests.rs`, `model/json/edit.rs`, `model/yaml/edit/tests.rs`.

## Sizes

`model/**` is ~21,500 lines across all three backends (TOML alone ~11,800); `schema/**` 983;
`session/**` ~9,000.
