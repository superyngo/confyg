# confyg design
Status: Draft

The design record for confyg: a schema-driven **Setter** for TOML / JSON(C) / YAML
configuration files, built on [confy](https://github.com/superyngo/confy)'s `confy-core`.

Vocabulary is fixed by [`../reference/glossary.md`](../reference/glossary.md); every bolded term
below has an entry there. Two decisions this design rests on are recorded separately:
[ADR 0001](../adr/0001-confy-core-as-pinned-git-dependency.md) (dependency strategy) and
[ADR 0002](../adr/0002-schema-driven-projection-and-three-state-presence.md)
(**Schema-driven projection** and **Presence**).

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
- **Notation switching.** No equivalent of confy's `K`. Whether a TOML table is a scope table or
  an inline table, whether an integer is hex or decimal, whether a YAML sequence is block or flow
  — all fixed by **Emission style** for new text, and left untouched for existing text.
- **Type conversion.** A **Field**'s type comes from the **Schema**. A value that does not match
  it produces a **Violation** and stays exactly as authored (**Soft constraint**).
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
which holds confy's editor machinery, is feature-gated off and never referenced.

```toml
confy-core = { git = "https://github.com/superyngo/confy", tag = "v1.0.1", default-features = false }
serde_json = { version = "1", features = ["preserve_order"] }
```

`preserve_order` is mandatory, not a preference. Without it `serde_json::Map` is a `BTreeMap` and
a Schema's `properties` arrive in alphabetical order, which would scramble every form into
`ca_cert, host, password, port, username` regardless of how the Schema author grouped them.

### What is reused, and what is new

| Concern | Source |
|---|---|
| Lossless parse / serialize, three **Doc formats**, byte-identical untouched regions | `confy-core` `model::` |
| **Path** addressing, value replacement, insert, delete | `confy-core` `model::` |
| JSON Schema 2020-12 validation, **Violation** → **Path** mapping, **Soft constraint** model | `confy-core` `schema::` |
| **Schema hint** detection for all three formats | `confy-core` `schema::hints` |
| `enum` / `const` / numeric-bound extraction, numeric clamping and stepping | `confy-core` `schema::hints_edit`, `session::schema_hint` (to be lifted or re-exported) |
| Undo / redo (full-text snapshot ring) | ported from confy's `session::undo_redo` |
| i18n catalog and lookup, design tokens, responsive chrome folding, file open/save, format conversion dialog | ported from confy's `i18n/` and `web/` |
| **Form IR**, its compiler, **Presence**, **Setter intents**, **Template** generation, form renderer | new |

confy's `web/render.ts` and `ViewRow` are not reused; ADR 0002 explains why.

## 3. Form IR

The single value every host renders. It carries no widget-toolkit or DOM concepts, so the Web,
and later TUI, hosts render the same tree.

```rust
enum FormNode {
    Field   { path: Path, widget: Widget, presence: Presence, meta: FieldMeta },
    Group   { path: Path, meta: NodeMeta, children: Vec<FormNode>, toggle: Option<GroupToggle> },
    Repeat  { path: Path, meta: NodeMeta, items: Vec<FormNode>,
              bounds: Bounds, item_template: TemplateRef, label_from: Option<String> },
    Map     { path: Path, meta: NodeMeta, entries: Vec<MapEntry>,
              bounds: Bounds, key_rule: KeyRule, value_template: TemplateRef },
    Variant { path: Path, meta: NodeMeta, options: Vec<VariantOption>,
              active: Option<usize>, discriminator: Option<Discriminator> },
    Tuple   { path: Path, meta: NodeMeta, slots: Vec<FormNode> },
    Unknown { path: Path, raw_preview: String },
    Cyclic  { path: Path, schema_ptr: String },
}

enum Presence {
    Absent  { default: Option<Value> },
    Set     { literal: String },
    Invalid { literal: String, violations: Vec<Violation> },
}

struct FieldMeta {
    title: String,               // Schema `title`, else the key humanized
    description: Option<String>,
    default: Option<Value>,
    examples: Vec<Value>,
    required: bool,
    deprecated: bool,
    read_only: bool,
    write_only: bool,
    unit: Option<String>,        // Annotation only; no Schema keyword carries this
    constraints: Vec<Constraint>,// what to render as guidance and check live
}
```

`Widget` is resolved from the Schema keywords governing the **Field**, in this precedence:
`const` / `readOnly` → display-only; `enum` → menu; **Annotation** override; `format` → the
matching specialized control; `type` → the primitive control. §7 lists every **Widget** and the
tier it lands in.

`Constraint` is the renderable subset of the validation vocabulary — bounds, `multipleOf`,
length, `pattern`, `uniqueItems` — kept on the **Field** so guidance text and live warnings do
not require a second Schema walk.

## 4. Compilation

`confyg-form` compiles `(Schema, Option<&NodeTree>) → FormNode` as a pure function. Order
matters:

1. **Resolve.** Follow `$ref` lazily, keeping a visited-pointer set; a pointer already on the
   current path compiles to **Cyclic stub** instead of recursing. Local `#/$defs/...` resolves
   in-process; an external or `https://` reference is requested from the host (§6).
2. **Merge.** Flatten `allOf` into one effective subschema. Conflicting keywords keep the
   narrowest constraint and record a diagnostic; they do not fail compilation.
3. **Evaluate conditionals.** Apply `if` / `then` / `else`, `dependentSchemas`, and
   `dependentRequired` against the *current* **Document** data. Conditional structure is
   therefore a function of the data, and setting a value can change which **Fields** exist. The
   compiler is re-run after every **Setter intent** that could flip a condition.
4. **Classify.** Choose the **Form node** kind: `properties` → **Group**; `array` of objects →
   **Repeat group**; `array` of scalars → a **Field** with a list **Widget**; `prefixItems` →
   **Tuple group**; `additionalProperties` / `patternProperties` → **Map group**; `oneOf` /
   `anyOf` of objects → **Variant group**; otherwise **Field**.
5. **Order.** Schema `properties` order as it appears in the schema file, overridable by an
   `x-order` **Annotation**. Required **Fields** are *not* hoisted — the Schema author's grouping
   is more meaningful than a required-first sort.
6. **Overlay.** For each leaf **Path**, look up the **Document** and assign **Presence**;
   attach **Violations** from `confy-core`'s validation pass by **Path**.
7. **Sweep unknowns.** Every **Document** key with no corresponding **Form node** is collected
   into one **Unknown group** at the end of its parent.

Compilation is the unit-test surface: it is deterministic, has no I/O, and takes only a Schema
and a document string.

## 5. Setter intents

Every intent lowers to a `confy-core` `Mutation` and is refused unless the **Schema** authorizes
it. Refusal is silent in the sense that the affordance is simply not offered — confyg does not
present an action and then reject it.

| Intent | Lowers to | Gate |
|---|---|---|
| `SetValue` | `Replace` | Always available on a writable **Field**. A value violating the Schema is written and warned about, never refused |
| `Unset` | `Delete` | Not offered when `required`, nor on an **Absent** **Field** |
| `AddRepeatItem` | `Insert` | `len < maxItems`; inserts the **Repeat group**'s item **Template** |
| `RemoveRepeatItem` | `Delete` | `len > minItems` |
| `MoveRepeatItem` | `Move` | **Repeat group** only; never a **Tuple group** |
| `AddMapEntry` | `Insert` | `len < maxProperties`; key must satisfy `propertyNames` / the matching pattern and not collide |
| `RenameMapKey` | `Rename` | **Map group** entries only — the sole user-authored key in confyg |
| `RemoveMapEntry` | `Delete` | `len > minProperties` |
| `SelectVariant` | `Replace` | **Variant group** only. Writes the chosen variant's **Template**; never migrates values between variants |
| `ToggleGroup` | `Insert` / `Delete` | **Group** not in `required`. Enabling writes the **Template**; disabling removes the whole section |
| `GenerateTemplate` | whole-document `Replace` | Only on an empty or absent **Document**. Takes a **Template strategy**: `required-only`, `with-defaults`, or `all-fields` |
| `LoadSchema` | — | Host-supplied Schema text, per `confy-core`'s existing fetch-request protocol |
| `Undo` / `Redo` | — | Full-text snapshot ring |

`SelectVariant` never migrating data is a policy, not a limitation to fix later: `oneOf`
discrimination without a discriminator keyword is unsolved in every surveyed form engine, and
the failure mode of guessing is silent data loss. Values belonging to the previous variant are
left in place and surfaced as **Violations**, so the user decides what to keep.

## 6. Schema resolution

`confy-core` performs no I/O, and confyg keeps that property: `confyg-session` emits a fetch
request and the host answers with `LoadSchema`. Resolution order:

1. An explicit Schema chosen by the user or passed on the command line.
2. The **Document**'s **Schema hint**.
3. A catalog match by filename (SchemaStore-style) — v0.2.
4. Nothing: the **Document** is shown as one **Unknown group**, read-only apart from removal,
   with a prompt to pick a Schema.

## 7. Configuration patterns

The catalogue this design must cover, with the release tier each lands in (§9). "Engine" notes
where `confy-core` already provides the mechanism and only the **Form IR** and renderer are
missing.

### Scalar patterns

| # | Pattern | Schema source | Widget | Tier |
|---|---|---|---|---|
| A1 | Free text | `type: string` | text | v0.1 |
| A2 | Constrained text | `pattern` | text + live regex check | v0.1 |
| A3 | Multiline text (certificate, key, script) | `contentMediaType`, long `maxLength` | textarea; YAML block scalar | v0.2 |
| A4 | Secret | `writeOnly` | masked with reveal | v0.2 |
| A5 | Single choice | `enum` | radio group ≤4 options, searchable menu above | v0.1 |
| A6 | Multiple choice | `array` + `items.enum` + `uniqueItems` | checkbox set / chips | v0.2 |
| A7 | Boolean | `type: boolean` | three-state control (ADR 0002) | v0.1 |
| A8 | Bounded number | `minimum` / `maximum` | stepper; slider when both bounds exist | v0.1 |
| A9 | Stepped number | `multipleOf` | stepper increment | v0.1 |
| A10 | Number with unit | **Annotation** | stepper + unit badge | v0.2 |
| A11 | Port | `integer` 1–65535 | port control | v0.3 |
| A12 | Date / time / duration | `format: date-time`, `date`, `time`, `duration` | picker; composite for duration | v0.2 |
| A13 | Path | `format: uri`, **Annotation** | text + native picker where the host has one | v0.3 |
| A14 | URL | `format: uri` | text + reachability probe | v0.3 |
| A15 | IP address | `format: ipv4` / `ipv6` | segmented text | v0.3 |
| A16 | UUID | `format: uuid` | text + generate | v0.3 |
| A17 | Regular expression | `format: regex` | monospace text + compile check | v0.3 |
| A18 | Colour | **Annotation** | swatch | v0.3 |
| A19 | Embedded file content | `contentEncoding: base64` | file picker | v0.3 |
| A20 | Nullable | `type: ["string","null"]` | control + explicit null. TOML has no null, so **Absent** is the only representation there | v0.2 |
| A21 | Fixed / read-only | `const`, `readOnly` | display-only badge | v0.1 |
| A22 | Suggested values | `examples` | suggestion chips | v0.2 |
| A23 | Interpolated value (`${VAR}`) | not expressible in Schema | text; the type mismatch is a **Violation** only | v0.1 |

### Container patterns

| # | Pattern | Schema source | Form node | Tier |
|---|---|---|---|---|
| B1 | Fixed section | `properties` | **Group** | v0.1 |
| B2 | Nested section | nested `properties` | nested **Group** | v0.1 |
| B3 | Optional whole section | absence from `required` | **Group** with `toggle` | v0.2 |
| B4 | Repeatable object entry | `array` of `object` | **Repeat group** — engine: `aot_group.rs` | v0.1 |
| B5 | User-named entries | `additionalProperties`, `patternProperties` | **Map group** — engine present | v0.2 |
| B6 | Scalar list | `array` of scalars | **Field** with list Widget | v0.1 |
| B7 | Fixed positions | `prefixItems` | **Tuple group** | v0.2 |
| B8 | Count bounds | `minItems` / `maxItems` | `(3/5)` badge; gates add and remove | v0.1 |
| B9 | Unique entries | `uniqueItems` | duplicate marking | v0.2 |
| B10 | Order-significant list | convention | move up / down | v0.2 |
| B11 | Variant | `oneOf` | **Variant group** | v0.3 |
| B12 | Scalar union | `anyOf` | type chooser + control | v0.3 |
| B13 | Schema merge | `allOf` | flattened at compile step 2 | v0.3 |
| B14 | Conditional fields | `if`/`then`/`else`, `dependentSchemas` | compile step 3 | v0.3 |
| B15 | Conditional requirement | `dependentRequired` | dynamic required marking | v0.3 |
| B16 | Recursive structure | `$ref: "#"` | **Cyclic stub** | v0.3 |
| B17 | Required vs optional | `required` | marker; optional-field add affordance | v0.1 |
| B18 | Unknown keys | absent from Schema | **Unknown group** | v0.1 |
| B19 | Deprecated field | `deprecated` | warning badge | v0.2 |
| B20 | Schema-less region | `additionalProperties: true` | read-only raw view | v0.1 |
| B21 | Out-of-subset YAML | anchors, aliases, merge keys | preserved read-only (engine) | v0.1 |

### Document patterns

| # | Pattern | Tier |
|---|---|---|
| C1 | Schema resolution: explicit → **Schema hint** → catalog → none (§6) | v0.1 (hint), v0.2 (catalog) |
| C2 | External and `https://` `$ref` resolution via the host | v0.2 |
| C3 | **Template** generation from a Schema with no **Document**, including the **Schema hint** | v0.1 |
| C4 | **Minimal write** | v0.1 |
| C5 | Emit to any of the three **Doc formats** | v0.1 |
| C6 | Violation summary with jump-to-field | v0.1 |
| C7 | Diff preview before write — against the file on disk, and against the **Template** | v0.2 |
| C8 | i18n of Schema `title` / `description` via an external table keyed by Schema path | v0.3 |
| C9 | Virtualized rendering for very large Schemas | v0.3 |
| C10 | Preset value sets (`dev` / `prod`) | v0.3 |

### Format hazards

| # | Hazard | Handling | Tier |
|---|---|---|---|
| D1 | **Table ordering rule** — TOML scalars must precede sub-tables | Every TOML insert resolves its slot accordingly; port confy's slot logic | v0.1 |
| D2 | TOML has two spellings for a **Repeat group** | **Emission style** fixes `[[servers]]` | v0.1 |
| D3 | TOML has no `null` | A20; **Absent** is the representation | v0.1 |
| D4 | Strict JSON has no comments | A `description` is never emitted as a comment; JSONC may | v0.1 |
| D5 | YAML block vs flow, indentation | **Emission style** fixes block | v0.1 |
| D6 | Schema `properties` order lost to `BTreeMap` | `serde_json` `preserve_order` (§2) | v0.1 |

## 8. Write policy

- **Minimal write.** A value equal to the Schema `default` is not written. Per ADR 0002 this is a
  correctness rule: writing it would move the **Field** from **Absent** to **Set** and pin it
  against future upstream default changes.
- **Emission style.** New text follows one fixed style per **Doc format** — TOML **Repeat group**
  as `[[name]]`, **Group** as a scope table, JSON as a multiline object or array, YAML as block
  mapping or sequence. Existing text is never restyled.
- **Table ordering rule.** A TOML insert places a scalar before any sub-table of its parent.
- **Insertion order.** A missing key is inserted at its Schema-declared position among its
  present siblings, not appended.
- **Schema hint.** A generated **Document** carries the hint in its format's convention.
- **Round-trip.** Comments, key order, and whitespace outside the touched span are preserved;
  untouched regions stay byte-identical, inherited from `confy-core`.

## 9. Release tiers

**v0.1** — the stated requirement. **Schema-driven projection** and **Presence**; **Template**
generation; menus, three-state booleans, bounded numbers; **Repeat group** with `+` and bounds;
scalar lists; required markers; **Unknown group**; violation summary; all three **Doc formats**;
every D-row hazard. Web host only.

**v0.2** — **Map group**, optional-section toggles, **Tuple group**, secrets, multiline, units,
multi-choice, date/time, examples, deprecation, order-significant lists, external `$ref`, catalog
matching, diff preview.

**v0.3** — **Variant group** and conditional structure (B11–B16), the specialized `format`
widgets, Schema-string i18n, virtualization, presets. TUI host.

`oneOf` and conditional structure are deliberately last: they are the parts every surveyed form
engine handles badly, and they should be designed against a settled **Form IR** rather than
shaping it.

## 10. Web host

Left: search over **Field** titles and descriptions, not only keys, plus section navigation.
Centre: the form. Right or bottom: the violation summary, and the diff preview at v0.2.

Every **Field** renders title, description, its control, **Ghost text** for an **Absent**
default, a unit badge where an **Annotation** supplies one, an **Unset** affordance when
**Set** or **Invalid**, and its warnings inline.

A **Repeat group** renders as a card list with `+ Add <item title>` and a `(3/5)` count badge on
its header, and per-card remove, duplicate, and move affordances. A card is titled from
`label_from` — the first of `name`, `id`, `title`, `host` the item schema declares — falling back
to `<title> #<n>`.

Ported wholesale from confy: the i18n catalog and lookup, the OKLCH token set and light/dark
themes, the responsive chrome-folding ladder, file open/save across browser / Tauri / VS Code
hosts, and the format-conversion dialog. UI behavior follows `wens-dev-principles ui`.

## 11. Verification

1. **Compiler snapshots.** `(schema, document) → Form IR` serialized and compared, one fixture
   set per **Doc format**, plus fixtures for each pattern in §7 as its tier lands.
2. **Round-trip tests.** `(schema, document, [Setter intent]) → expected bytes`. These are the
   tests that pin **Minimal write**, **Emission style**, the **Table ordering rule**, and
   byte-identical preservation of untouched regions.
3. **Presence matrix.** For every **Widget**, the three **Presence** states and the transitions
   between them, including that **Unset** deletes rather than writes.
4. **Real-binary check.** Against a real published Schema, exercise open → add a **Repeat group**
   item → set values → **Unset** one → save, on the actual web build. Green unit tests are not
   evidence that the flow works.

## 12. Open questions

- Whether `confy-core` should expose `schema::hints_edit` extensions (`default`, `required`,
  `title`, `examples`, `deprecated`, `readOnly` / `writeOnly`, `prefixItems`,
  `additionalProperties`) upstream so confy's hover benefits too, or whether confyg keeps its own
  Schema introspection in `confyg-form`. Upstream is preferable if the API stays generic.
- The **Annotation** vocabulary is not yet enumerated. It must be specified before v0.2, since
  A10, A13, and A18 depend on it. It stays optional in every case.
- Whether `MoveRepeatItem` is worth v0.1 or belongs with B10 in v0.2.
