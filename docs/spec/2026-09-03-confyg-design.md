# confyg design
Status: Draft

The design record for confyg: a schema-driven **Setter** for TOML / JSON(C) / YAML
configuration files, built on [confy](https://github.com/superyngo/confy)'s `confy-core`.

Vocabulary is fixed by [`../reference/glossary.md`](../reference/glossary.md); every bolded term
below has an entry there. Facts about the upstream dependency — the pin, the reachable API, index
spaces, validator behaviour — are owned by [`../reference/upstream.md`](../reference/upstream.md)
and are cited, not restated, here. Three decisions this design rests on are recorded separately:
[ADR 0001](../adr/0001-confy-core-as-pinned-git-dependency.md) (dependency strategy),
[ADR 0002](../adr/0002-schema-driven-projection-and-three-state-presence.md)
(**Schema-driven projection** and **Presence**), and
[ADR 0003](../adr/0003-form-ir-shape-presence-scope-and-default-equivalence.md)
(**Presence** scope, collection unification, default equivalence). How a **Form IR** becomes
something a person looks at is a separate design record,
[`2026-09-04-presentation-layers-design.md`](2026-09-04-presentation-layers-design.md), resting on
[ADR 0004](../adr/0004-presentation-layer-model-and-write-neutrality.md) (the layer model and
**Write-neutrality**) and
[ADR 0005](../adr/0005-presentation-profile-as-a-second-carrier.md) (one vocabulary, two
carriers).

---

## 1. Purpose

Given a JSON Schema and, optionally, an existing config file, render a form that makes the file
correct to fill in: menus instead of free text where the Schema says `enum`, a `+` on a
**Repeat group** that adds a whole server entry with its defaults, array entries bounded by
`minItems` / `maxItems`, inherited defaults visible without being written, and a wrong value
warning rather than blocking.

### Non-goals

These are deliberate omissions, not deferred work:

- **Arbitrary structural editing.** The user cannot insert, delete, move, or rename a **Node** of
  their choosing. Every write is a **Setter intent** the **Schema** authorized. The one exception
  is a **Map group**, whose keys are user-authored by definition.
- **Notation switching.** No equivalent of confy's `K`; `Mutation::ConvertKind` is never used.
  Whether a TOML table is a scope table or an inline table, whether an integer is hex or decimal,
  whether a YAML sequence is block or flow — all fixed by **Emission style** for new text, and
  left untouched for existing text. Whole-file format conversion is a different thing: it
  re-emits the entire **Document**, and it is a **Session command** (§5).
- **Type conversion.** A **Field**'s type comes from the **Schema**. A value that does not match
  it produces a **Violation** and stays exactly as authored (**Soft constraint**), reachable
  through **Raw literal fallback**.
- **Free-text document editing.** No `$EDITOR` handoff, no editing of raw syntax. A raw view is
  read-only, and exists for inspection and diff only.
- **Multi-file workspaces, config layering, profile merging.** One **Document** per session.

## 2. Architecture

```
confyg/
├─ crates/
│  ├─ confyg-form/       Schema → Form IR compiler. Pure functions, no I/O, no state.
│  ├─ confyg-session/    Presence overlay, Setter intent dispatch, Template generation.
│  └─ confyg-ffi/        WASM boundary: dispatch(SetterIntent) → SetterSnapshot.
├─ web/                  Form renderer and host glue.
└─ i18n/{en,zh-TW}.json
```

`confy-core` supplies the **Document** layer (`model::`) and the Schema layer (`schema::`) as a
pinned git dependency with `default-features = false` — see ADR 0001. Its `session::` module,
which holds confy's editor machinery, is feature-gated off and never referenced; that gating is
verified safe, since neither `model` nor `schema` refers to `session`.

The exact pin, the dependency table, and the two upstream changes confyg requires are in
[`upstream.md`](../reference/upstream.md). Two of its findings shape this design directly:
`serde_json`'s `preserve_order` is mandatory and safe to enable globally (without it a Schema's
`properties` arrive alphabetically and every form is scrambled into
`ca_cert, host, password, port, username`), and `Target.index` is **not** the index space a
**Path** uses (§8).

### What is reused, and what is new

| Concern | Source |
|---|---|
| Lossless parse / serialize, three **Doc formats**, byte-identical untouched regions | `confy-core` `model::` |
| **Path** addressing, value replacement, insert, delete, move | `confy-core` `model::`, via the public `ConfigDocument` trait |
| JSON Schema validation, **Violation** → **Path** mapping, **Soft constraint** model | `confy-core` `schema::validate`, `schema::value_bridge` |
| **Schema hint** detection for all three formats | `confy-core` `schema::hints::detect_hint` |
| Whole-document format conversion | `confy-core` `model::convert` |
| `enum` / `const` / numeric-bound extraction, numeric clamping and stepping | new in `confyg-form` — `schema::hints_edit` reads none of the keywords confyg needs, and `session::schema_hint` is numeric nudging behind the disabled feature |
| Schema keyword introspection (`default`, `required`, `title`, `examples`, `deprecated`, `readOnly` / `writeOnly`, `prefixItems`, `additionalProperties`) | new in `confyg-form` |
| Insertion-ordinal mapping (projection index → child ordinal) | ported from `session::insertion` for v0.1; requested upstream as `model::insert_ordinal` |
| Undo / redo (full-text snapshot ring) | ported from confy's `session::undo_redo` |
| Schema fetch request / response | new in `confyg-session`, reusing `SchemaSource` only |
| i18n catalog and lookup, design tokens, responsive chrome folding, file open/save, format conversion dialog | ported from confy's `i18n/` and `web/` |
| **Form IR**, its compiler, **Presence**, **Setter intents**, **Template** generation, form renderer | new |

confy's `web/render.ts` and `ViewRow` are not reused; ADR 0002 explains why.

## 3. Form IR

The single value every host renders. It carries no widget-toolkit or DOM concepts, so the Web,
and later TUI, hosts render the same tree.

```rust
enum FormNode {
    Field   { path: Path, widget: Widget, presence: Presence, meta: FieldMeta },
    Group   { path: Path, meta: NodeMeta, children: Vec<FormNode>,
              occupancy: Occupancy, toggle: Option<GroupToggle> },
    Repeat  { path: Path, meta: NodeMeta, items: Vec<FormNode>, occupancy: Occupancy,
              bounds: Bounds, item_template: TemplateRef, label_from: Option<String> },
    Map     { path: Path, meta: NodeMeta, entries: Vec<MapEntry>, occupancy: Occupancy,
              bounds: Bounds, key_rule: KeyRule, value_template: TemplateRef },
    Variant { path: Path, meta: NodeMeta, options: Vec<VariantOption>,
              active: Option<usize>, discriminator: Option<Discriminator> },
    Union   { path: Path, meta: NodeMeta, presence: Presence,
              options: Vec<UnionOption>, active: Option<usize> },
    Tuple   { path: Path, meta: NodeMeta, slots: Vec<FormNode>, occupancy: Occupancy },
    Unknown { path: Path, raw_preview: String },
    Cyclic  { path: Path, schema_ptr: String },
}

enum Presence {
    Absent  { default: Option<Value>, remarked: Option<String> },
    Set     { literal: String },
    Invalid { literal: String, violations: Vec<Violation> },
}

enum Occupancy { Absent, Empty, Populated }

struct NodeMeta {
    title: String,
    description: Option<String>,
    violations: Vec<Violation>,   // this node's own failures, not its children's
    locked: Option<Locked>,       // alias, merge-key inheritance: rendered, never written
    deprecated: bool,
}

struct FieldMeta {
    // NodeMeta's fields, plus:
    default: Option<Value>,
    examples: Vec<Value>,
    required: bool,
    read_only: bool,
    write_only: bool,
    unit: Option<String>,        // Annotation only; no Schema keyword carries this
    intended: Widget,            // what derivation chose, before the Host capability clamp
    constraints: Vec<Constraint>,// what to render as guidance and check live
    raw: bool,                   // Raw literal fallback is active on this Field
}
```

Per ADR 0003: **Presence** is a **Field** concept and stays three-state; every node carries its
own `violations`, because the validator reports `minItems`, `required`, `uniqueItems`,
`minProperties`, and `additionalProperties` against the *container's* **Path**; and every
container carries an **Occupancy**, because `servers = []` and a missing `servers` key are
different facts. `Presence::Absent` reserves `remarked` for v0.2 (§9).

`Union` is the **Union field** — a scalar `anyOf`, where the user chooses a type and then a value.
It is a **Field**-like node rather than a **Variant group** because it holds one value.

`Widget` is resolved from the Schema keywords governing the **Field**, in this precedence:
`const` / `readOnly` → display-only; **Raw literal fallback** → raw text; **Annotation** or
**Presentation profile** override; `enum` → the menu family; `format` → the matching specialized
control; `type` → the primitive control. The resolved **Widget** is then clamped along its
**Degradation ladder** to what the host declared it can render, and `intended` keeps the
pre-clamp choice so a substitution can be explained. §7 lists every **Widget** and the tier it
lands in;
[`2026-09-04-presentation-layers-design.md`](2026-09-04-presentation-layers-design.md) owns the
**Affordance** layer — the override vocabulary, the menu-family thresholds, and every ladder.

`Constraint` is the renderable subset of the validation vocabulary — bounds, `multipleOf`,
length, `pattern`, `uniqueItems` — kept on the **Field** so guidance text does not require a
second Schema walk. It is guidance only: the authority for whether a value violates the Schema is
always the validator (§7).

A **Schema** has two independent states, and both are surfaced: *projected* (compiled into a
**Form IR**) and *validatable* (`jsonschema::Validator::new` succeeded). One uncompilable
`pattern` costs the whole **Document** its validation, so the second state is modelled explicitly
— reusing `confy-core`'s `SchemaState` — never left to look like "no problems found".

## 4. Compilation

`confyg-form` compiles `(Schema, Option<&NodeTree>) → FormNode` as a pure function. Order
matters:

0. **Normalize.** Rewrite Draft 7 spellings to their 2020-12 equivalents — `items` as an array →
   `prefixItems`, `definitions` → `$defs`, `dependencies` → `dependentSchemas` /
   `dependentRequired`, boolean `exclusiveMinimum` / `exclusiveMaximum`, `additionalItems`. v0.2;
   until then a Draft 7 Schema validates correctly but projects imperfectly (§6).
1. **Resolve.** Follow `$ref` lazily, keeping a visited-pointer set; a pointer already on the
   current path compiles to **Cyclic stub** instead of recursing. Local `#/$defs/...` resolves
   in-process; an external or `https://` reference is requested from the host (§6).
2. **Merge.** Flatten `allOf` into one effective subschema. Conflicting keywords keep the
   narrowest constraint and record a diagnostic; they do not fail compilation.
3. **Evaluate conditionals.** Apply `if` / `then` / `else`, `dependentSchemas`, and
   `dependentRequired` against the *current* **Document** data. Conditional structure is
   therefore a function of the data, and setting a value can change which **Fields** exist. The
   compiler is re-run after every **Setter intent**; in-progress text never participates, because
   it is not in the **Document** (§5).
4. **Classify.** Choose the **Form node** kind: `properties` → **Group**; `array` → **Repeat
   group**, whose items are **Groups** for an `object` item schema and **Fields** for a scalar one
   (ADR 0003); `prefixItems` → **Tuple group**; `additionalProperties` / `patternProperties` as a
   *schema* → **Map group**; `oneOf` / `anyOf` of objects → **Variant group**; `anyOf` of scalars
   → **Union field**; otherwise **Field**.
5. **Order.** Schema `properties` order as it appears in the schema file, overridable by
   `x-confyg.order`. Required **Fields** are *not* hoisted — the Schema author's grouping
   is more meaningful than a required-first sort. This is the **Form IR**'s order and the order
   the UI shows; the **Document**'s order may legitimately differ (§8).
6. **Overlay.** For every **Path** — leaf and container alike — look up the **Document**, assign
   **Presence** or **Occupancy**, and attach that node's own **Violations** by exact **Path**. A
   **Node** whose text is a YAML alias, or whose value arrives through a merge key, is marked
   `locked`. Two attribution rules follow from how `confy-core` maps pointers: a **Violation**
   whose pointer resolves to an ancestor attaches to that ancestor, not to the intended child,
   and an unresolvable pointer attaches to the root.
7. **Sweep unknowns.** Every **Document** key with no corresponding **Form node** is collected
   into one **Unknown group** at the end of its parent, per the decision table in §7 (B18).

Compilation is the unit-test surface: it is deterministic, has no I/O, and takes only a Schema
and a document string. It does not require a validator, so a **Schema** that cannot be compiled
for validation still produces a complete form.

## 5. Setter intents and Session commands

Two vocabularies. A **Setter intent** mutates the **Document** and is Schema-gated; a **Session
command** operates on the session and is not. Refusal is silent in the sense that the affordance
is simply not offered — confyg does not present an action and then reject it.

Every intent passes `OnCollision::Cancel` and, where a key must be synthesized, an explicit
`suggested_key`: confyg never lets the engine invent a key or resolve a conflict on its behalf, so
a generic `placeholder` key can never appear in a confyg-written file.

| Intent | Lowers to | Gate |
|---|---|---|
| `SetValue` | `Replace` | Always available on a writable **Field**. A value violating the Schema is written and warned about, never refused. A value equal to the effective `default` lowers to `Delete` instead (ADR 0003) |
| `Unset` | `Delete` | Not offered when `required`, nor on an **Absent** **Field** |
| `AddRepeatItem` | `Insert` | `len < maxItems`. Two lowerings: into an existing collection, a headerless fragment addressed at the collection's **Path**; into an **Absent** one, a header-bearing fragment addressed at the parent (§8) |
| `RemoveRepeatItem` | `Delete` | `len > minItems` |
| `AddMapEntry` | `Insert` | `len < maxProperties`; key must satisfy `propertyNames` / the matching pattern and not collide. A colliding key is refused inline on the key input, never auto-renamed. Same two lowerings as `AddRepeatItem` |
| `RenameMapKey` | `Rename` | **Map group** entries only — the sole user-authored key in confyg. Refused onto an occupied key |
| `RemoveMapEntry` | `Delete` | `len > minProperties` |
| `SelectVariant` | `Replace` | **Variant group** only. Writes the chosen variant's **Template**; never migrates values between variants |
| `SelectUnionType` | `Replace` | **Union field** only. Clears to **Absent** rather than coercing the existing literal |
| `ToggleGroup` | `Insert` / `Delete` | **Group** not in `required`. Enabling writes the **Template**; disabling removes the whole section. Same two lowerings as `AddRepeatItem` |
| `GenerateTemplate` | whole-document `Replace` | Only on an empty or absent **Document**. Takes `target: DocFormat`, a **Template strategy**, and a **Comment policy** |
| `MoveRepeatItem` | `Move` | **Repeat group** only; never a **Tuple group**. v0.2 (§9) |

Every additive intent has an **absent-parent lowering**: the first item of a collection, the first
entry of a map, and the enabling of an optional group all create the container as well as the
member, and are therefore a different mutation from the second one. This is the general rule, not
three special cases.

| Session command | Notes |
|---|---|
| `Open` / `Save` | Host file glue, ported from confy's `web/` |
| `ConvertFormat` | Whole-**Document** re-emission via `model::convert`; not notation switching (§1) |
| `LoadSchema` | Answers `confyg-session`'s fetch request (§6) |
| `Undo` / `Redo` | Full-text snapshot ring; one entry per committed intent, not per keystroke |

**In-progress text is not a Presence.** The host owns the edit buffer; a keystroke becomes a
`SetValue` on commit (blur or Enter), never per character. The **Document** therefore records
committed decisions only, compilation stays deterministic, and the snapshot ring stays meaningful.
Live guidance runs against the buffer, but a **Violation** only ever describes written text.

`SelectVariant` never migrating data is a policy, not a limitation to fix later: `oneOf`
discrimination without a discriminator keyword is unsolved in every surveyed form engine, and
the failure mode of guessing is silent data loss. Values belonging to the previous variant are
left in place and surfaced as **Violations**, so the user decides what to keep.

## 6. Schema resolution

`confy-core` performs no I/O, and confyg keeps that property: `confyg-session` emits a fetch
request and the host answers with `LoadSchema`. The protocol is confyg's own — confy's lives in
the disabled `session::` module — and reuses only `SchemaSource` and `schema::hints::detect_hint`.
Naming deliberately mirrors confy's (`schema_fetch_request`, `SchemaLoaded`) so the two codebases
read alike. Resolution order:

1. An explicit Schema chosen by the user or passed on the command line.
2. The **Document**'s **Schema hint**.
3. A catalog match by filename (SchemaStore-style) — v0.2.
4. Nothing: the **Document** is shown as one **Unknown group**, read-only apart from removal,
   with a prompt to pick a Schema.

A Schema with no `$schema` member is validated as 2020-12. A Draft 7 Schema is validated under
Draft 7 semantics by `jsonschema`, but until step 0 of §4 lands its structural keywords are
unrecognized by the compiler, so the form is best-effort.

## 7. Configuration patterns

The catalogue this design must cover, with the release tier each lands in (§9). "Engine" notes
where `confy-core` already provides the mechanism and only the **Form IR** and renderer are
missing.

Two invariants govern this whole section. *Form warnings and Violations never disagree*: the
validator is the single authority on whether a value breaks the Schema, so live checks call the
same engine through the WASM boundary rather than reimplementing it host-side. And **Raw literal
fallback**: a **Field** whose literal does not parse as its **Widget**'s type renders as raw text
with its **Violation**, offering the typed control once the literal parses; the user may also
enter raw mode deliberately, which is what makes A23 possible.

### Scalar patterns

| # | Pattern | Schema source | Widget | Tier |
|---|---|---|---|---|
| A1 | Free text | `type: string` | text | v0.1 |
| A2 | Constrained text | `pattern` | text + live check via the validator's engine (`fancy-regex`; lookaround and backreferences accepted) | v0.1 |
| A3 | Multiline text (certificate, key, script) | `contentMediaType`, long `maxLength` | textarea; YAML block scalar | v0.2 |
| A4 | Secret | `writeOnly` | masked with reveal; where the host cannot mask, it degrades to plain text with an explicit **Notice**, never silently | v0.2 |
| A5 | Single choice | `enum` | `radio` ≤4 options, `menu` 5–12, `filterable-menu` above; the `default` appears as the inherited option | v0.1 |
| A6 | Multiple choice | `array` + `items.enum` + `uniqueItems` | **Repeat group** with a checkbox-set Widget (ADR 0003) | v0.2 |
| A7 | Boolean | `type: boolean` | three-state control (ADR 0002) | v0.1 |
| A8 | Bounded number | `minimum` / `maximum` | stepper; slider when both bounds exist | v0.1 |
| A9 | Stepped number | `multipleOf` | stepper increment | v0.1 |
| A10 | Number with unit | `x-confyg.unit` | stepper + unit badge | v0.2 |
| A11 | Port | `integer` 1–65535 | port control | v0.3 |
| A12 | Date / time / duration | `format: date-time`, `date`, `time`, `duration` | picker; composite for duration | v0.2 |
| A13 | Path | `format: uri`, `x-confyg.affordance` | text + native picker where the host has one | v0.3 |
| A14 | URL | `format: uri` | text + reachability probe | v0.3 |
| A15 | IP address | `format: ipv4` / `ipv6` | segmented text | v0.3 |
| A16 | UUID | `format: uuid` | text + generate | v0.3 |
| A17 | Regular expression | `format: regex` | monospace text + compile check via the validator's engine | v0.3 |
| A18 | Colour | `x-confyg.affordance` | swatch | v0.3 |
| A19 | Embedded file content | `contentEncoding: base64` | file picker | v0.3 |
| A20 | Nullable | `type: ["string","null"]` | control + explicit null. TOML has no null, so **Absent** is the only representation there | v0.2 |
| A21 | Fixed / read-only | `const`, `readOnly` | display-only badge; **Unset** is offered on a `readOnly` **Field** that is **Set**, since removing a value confyg may not write is still legal | v0.1 |
| A22 | Suggested values | `examples` | suggestion chips | v0.2 |
| A23 | Interpolated value (`${VAR}`) | not expressible in Schema | **Raw literal fallback**; the type mismatch is a **Violation** only | v0.1 |

**Widget** names in this table are the ones the **Degradation ladder** table names, and each row's
tier is when its *ideal* control lands; the ladder's terminal control is available from v0.1 in
every case. Both live in
[`2026-09-04-presentation-layers-design.md`](2026-09-04-presentation-layers-design.md) §3–§4.

### Container patterns

| # | Pattern | Schema source | Form node | Tier |
|---|---|---|---|---|
| B1 | Fixed section | `properties` | **Group** | v0.1 |
| B2 | Nested section | nested `properties` | nested **Group** | v0.1 |
| B3 | Optional whole section | absence from `required` | **Group** with `toggle`, **Occupancy** `Absent` | v0.2 |
| B4 | Repeatable object entry | `array` of `object` | **Repeat group**; engine: `Insert` with a synthesized header (§8) | v0.1 |
| B5 | User-named entries | `additionalProperties` / `patternProperties` as a *schema* | **Map group** | v0.2 |
| B6 | Scalar list | `array` of scalars | **Repeat group** of **Fields** (ADR 0003) | v0.1 |
| B7 | Fixed positions | `prefixItems` | **Tuple group**; a trailing `items` schema compiles the variable tail as a **Repeat group** sibling | v0.2 |
| B8 | Count bounds | `minItems` / `maxItems` | `(3/5)` badge; gates add and remove; the **Violation** attaches to the collection | v0.1 |
| B9 | Unique entries | `uniqueItems` | duplicate marking | v0.2 |
| B10 | Order-significant list | convention | move up / down | v0.2 |
| B11 | Variant | `oneOf` | **Variant group** | v0.3 |
| B12 | Scalar union | `anyOf` of scalars | **Union field** | v0.3 |
| B13 | Schema merge | `allOf` | flattened at compile step 2 | v0.3 |
| B14 | Conditional fields | `if`/`then`/`else`, `dependentSchemas` | compile step 3 | v0.3 |
| B15 | Conditional requirement | `dependentRequired` | dynamic required marking | v0.3 |
| B16 | Recursive structure | `$ref: "#"` | **Cyclic stub** | v0.3 |
| B17 | Required vs optional | `required` | marker; optional-field add affordance | v0.1 |
| B18 | Unknown keys | see decision table below | **Unknown group** | v0.1 |
| B19 | Deprecated field | `deprecated` | warning badge | v0.2 |
| B20 | Empty vs absent collection | — | **Occupancy** distinguishes them (ADR 0003) | v0.1 |
| B21 | YAML aliases, anchors, merge keys | out of the Schema's reach | `locked`: resolved value shown, no write affordance, one explanatory notice | v0.1 |

`additionalProperties` means three different things and is decided by the keyword's *form*, not by
three overlapping rows:

| Form of the keyword | Result |
|---|---|
| a schema object | **Map group** — user-authored keys (B5). Until v0.2, renders as an **Unknown group** |
| `true`, or absent | **Unknown group** — preserved, read-only apart from removal, no **Violation** |
| `false` | **Unknown group**, plus the `additionalProperties` **Violation** the validator actually reported, attached to the containing node |

An **Unknown key** carries a **Notice**, not a **Violation**, unless the validator genuinely
reported one: under `additionalProperties: true` an extra key breaks no rule, and confyg does not
fabricate failures. Because the validator names offending keys in the message only, a `false`
**Violation** attaches to the container and the notice names the key.

### Document patterns

| # | Pattern | Tier |
|---|---|---|
| C1 | Schema resolution: explicit → **Schema hint** → catalog → none (§6) | v0.1 (hint), v0.2 (catalog) |
| C2 | External and `https://` `$ref` resolution via the host | v0.2 |
| C3 | **Template** generation from a Schema with no **Document**, including the **Schema hint** | v0.1 |
| C4 | **Minimal write** | v0.1 |
| C5 | Emit to any of the three **Doc formats** | v0.1 |
| C6 | Violation summary with jump-to-node; "validation unavailable" when the validator failed to compile, never "no problems" | v0.1 |
| C7 | Diff preview before write — against the file on disk, and against the **Template** | v0.2 |
| C8 | i18n of Schema `title` / `description` via an external table keyed by Schema path | v0.3 |
| C9 | Virtualized rendering for very large Schemas | v0.3 |
| C10 | Preset value sets (`dev` / `prod`) | v0.3 |
| C11 | Draft 7 normalization (§4 step 0) | v0.2 |

### Hazards

| # | Hazard | Handling | Tier |
|---|---|---|---|
| D1 | **Table ordering rule** — TOML scalars must precede sub-tables | Inserts into a sub-table are clamped by the engine; at the document root an illegal index is an error, so confyg computes the root slot itself | v0.1 |
| D2 | TOML has two spellings for a **Repeat group** | The `Target` chosen decides it: addressing the collection's **Path** makes the engine synthesize `[[servers]]`; a header fragment is rejected there (§8) | v0.1 |
| D3 | TOML has no `null` | A20; **Absent** is the representation | v0.1 |
| D4 | Strict JSON has no comments | **Comment policy**, derived per **Document** (§8) | v0.1 |
| D5 | YAML block vs flow, indentation | **Emission style** fixes block | v0.1 |
| D6 | Schema `properties` order lost to `BTreeMap` | `serde_json` `preserve_order` (§2) | v0.1 |
| D7 | `Target.index` counts comments; `Seg::Index` does not | Every `Insert` converts projection index → child ordinal first; misplacement, not an error, is the failure mode | v0.1 |
| D8 | One uncompilable `pattern` disables all validation | Modelled as a **Schema** state; the form still renders, the summary says so (§3, C6) | v0.1 |
| D9 | A wrong fragment shape writes a different structure and reports success | Fully-formed fragments plus explicit `suggested_key`; the post-mutation **Document** must recompile to the shape the intent predicted (§11) | v0.1 |

## 8. Write policy

- **Minimal write.** A value equal to the Schema `default` is not written, and `SetValue` with
  such a value deletes the key. Per ADR 0002 and ADR 0003 this is a correctness rule, not an
  optimization: writing it would move the **Field** from **Absent** to **Set** and pin it against
  future upstream default changes.
- **Emission style.** New text follows one fixed style per **Doc format** — **Group** as a scope
  table, JSON as a multiline object or array, YAML as block mapping or sequence. For a TOML
  **Repeat group** the style is expressed as a `Target` choice rather than as fragment text: a
  headerless fragment addressed at the collection's **Path** makes the engine write `[[name]]`,
  and passing a header there is rejected. Existing text is never restyled.
- **Absent-parent lowering.** The first member of an **Absent** collection cannot be addressed
  inside it, because it does not exist: it is inserted as a header-bearing fragment into the
  parent, subject to the **Table ordering rule**. Every later member uses the ordinary lowering.
- **Insertion order.** A missing key is inserted at its Schema-declared position among its present
  siblings, *subject to the **Table ordering rule***, and never appended blindly.
- **Order divergence.** Where the two conflict — a Schema declaring a scalar after an object, in
  TOML — legality wins. The **Form IR**'s order is authoritative for the UI; the **Document**'s
  order is whatever the format permits, and confyg never reorders existing text to chase the
  Schema.
- **Ordinal conversion.** `Target.index` is an ordinal in the parent's full child sequence,
  comments included, and for `Move` it is a pre-deletion ordinal. Every insertion converts from
  the projection index first — see [`upstream.md`](../reference/upstream.md).
- **Comment policy.** Derived per **Document**, never chosen: `.jsonc` / `.json5` allow comments,
  `.json` does not, and a `.json` file that arrived *with* comments is widened to allow them,
  since its consumer demonstrably tolerates them. A new file with no extension denies until the
  format dialog sets one. TOML and YAML always allow. Comments are emitted **only** by
  `GenerateTemplate`; an existing **Document**'s comments are never touched.
- **Schema hint.** A generated **Document** carries the hint in its format's convention.
- **Round-trip.** Comments, key order, and whitespace outside the touched span are preserved;
  untouched regions stay byte-identical, inherited from `confy-core`.

## 9. Release tiers

**v0.1** — the stated requirement. **Schema-driven projection**, **Presence** and **Occupancy**;
**Template** generation with `title` / `description` comments; menus, three-state booleans,
bounded numbers; **Repeat group** with `+` and bounds, including scalar lists; **Raw literal
fallback**; required markers; **Unknown group**; violation summary; all three **Doc formats**;
every D-row hazard. Presentation: the eight layers and **Write-neutrality**, the **Presentation
vocabulary** as `x-confyg`, **Host capability** with compile-time clamping, `scroll` and
`sections`, **Form search**, and the added **Appearance tokens**. Web host only.

**v0.2** — **Map group**, optional-section toggles, **Tuple group**, secrets, multiline, units,
multi-choice, date/time, examples, deprecation, order-significant lists and `MoveRepeatItem`,
Draft 7 normalization, external `$ref`, catalog matching, diff preview, and `Remark` as a
value-preserving alternative to **Unset** (`Presence::Absent { remarked }`). Presentation: the
**Presentation profile** sidecar with its **Profile hint** and sibling discovery, `tabs`, the
combobox that implements `filterable-menu`, and the **Lexicon** translation tables.

**v0.3** — **Variant group**, **Union field**, and conditional structure (B11–B16), the
specialized `format` widgets, Schema-string i18n, virtualization, presets. Presentation:
`wizard` and the TUI's **Host capability** profile. TUI host.

`oneOf` and conditional structure are deliberately last: they are the parts every surveyed form
engine handles badly, and they should be designed against a settled **Form IR** rather than
shaping it. `MoveRepeatItem` is deliberately not v0.1: `Move`'s pre-deletion ordinal in
comment-inclusive space is the most error-prone corner of the mutation API, nothing in the v0.1
requirement asks for reordering, and it is better added once D7 is pinned by fixtures.

## 10. Web host

Left: **Form search** over **Field** titles and descriptions, not only keys, plus **Partition**
navigation. Centre: the form. Right or bottom: the violation summary, and the diff preview at
v0.2. A persistent banner names the keyword and pointer when the **Schema** could not be compiled
for validation.

Every **Field** renders title, description, its control, **Ghost text** for an **Absent**
default, a unit badge where `x-confyg.unit` supplies one, its warnings inline, and — where the
control can express it — the Schema `default` as an explicit inherited option, which is how
**Unset** is offered. A control that cannot express its default keeps a separate **Unset**
affordance. A `locked` node renders its resolved value with an explanatory notice and no write
affordance.

A **Repeat group** renders as a card list with `+ Add <item title>` and a `(3/5)` count badge on
its header, and per-card remove and duplicate affordances (move at v0.2). A card is titled from
`x-confyg.labelFrom`, defaulting to the first of `name`, `id`, `title`, `host` the item schema
declares and falling back to `<title> #<n>`. A **Repeat group** of scalars renders as rows rather
than cards.

Ported wholesale from confy: the i18n catalog and lookup, the OKLCH token set and light/dark
themes, the responsive chrome-folding ladder, file open/save across browser / Tauri / VS Code
hosts, and the format-conversion dialog. confy's data-type colours are deliberately *not*
inherited, and confyg adds the **Appearance tokens** its own states need. UI behavior follows
`wens-dev-principles ui`; **Partition**, **Traversal**, and every **Degradation ladder** are owned
by [`2026-09-04-presentation-layers-design.md`](2026-09-04-presentation-layers-design.md).

## 11. Verification

1. **Compiler snapshots.** `(schema, document, profile, HostProfile) → Form IR` serialized and
   compared, one fixture set per **Doc format** and one per **Host capability** profile, so a
   clamped **Widget** and its retained `intended` are asserted rather than inspected. Plus
   fixtures for each pattern in §7 as its tier lands. Includes a
   Schema whose `pattern` cannot compile, which must still project a complete form.
2. **Round-trip matrix.** `(schema, document, [Setter intent]) → expected bytes`, covering
   **intent × target node kind × Doc format** rather than one fixture set per format. These are
   the tests that pin **Minimal write**, **Emission style**, the **Table ordering rule**, and
   byte-identical preservation of untouched regions. Three cases are mandatory in every format:
   - *Absent parent*: absent collection → first item → second item, per **Absent-parent
     lowering**.
   - *Comments interleaved* between every sibling, which is what catches D7's misplacement.
   - **Order divergence**: a Schema declaring a scalar after an object, in TOML.
3. **Presence and Occupancy matrix.** For every **Widget** — including each **Degradation
   ladder**'s terminal control, which must express all three states too — the three **Presence**
   states and the transitions between them, including that **Unset** deletes rather than writes
   and that setting a value equal to the `default` is **Unset**; for every container, absent →
   empty → populated and back.
4. **Intent postcondition.** An intent counts as verified only when the mutated **Document**
   recompiles to the **Form IR** shape the intent predicted — cheap, since §4 recompiles anyway
   and `apply` returns the new text. This is the guard against D9.
5. **Real-binary check.** Against a real published Schema, exercise open → add a **Repeat group**
   item → set values → **Unset** one → save, on the actual web build. Green unit tests are not
   evidence that the flow works.
6. **Write-neutrality.** For a fixed `(schema, document, [Setter intent])`, the whole pipeline run
   under *N* presentation profiles × *M* **Host capability** profiles must produce byte-identical
   output. A property over presentation inputs rather than another fixture matrix: it asserts that
   nothing in the **Affordance**, **Flow**, **Lexicon**, or **Appearance** layer can reach the
   file, and pins **Host capability** as pure data as a side effect.

## 12. Open questions

- None outstanding for the **Annotation** vocabulary. Two remain in
  [`2026-09-04-presentation-layers-design.md`](2026-09-04-presentation-layers-design.md) §10: the
  `Density` mapping onto confy's row-height scales, and the carrier for preset value sets (C10).

Resolved since drafting: `confy-core`'s Schema introspection stays in `confyg-form` rather than
being upstreamed, since `schema::hints_edit` reads none of the eight keywords confyg needs and
this is pure computation on confyg's own cadence (see [`upstream.md`](../reference/upstream.md));
`MoveRepeatItem` belongs to v0.2 with B10 (§9).

The **Annotation** vocabulary is likewise resolved: it is the nine-member **Presentation
vocabulary**, carried by `x-confyg` and by a **Presentation profile** (ADR 0005), and
`label_from` became `x-confyg.labelFrom` as that question anticipated.
