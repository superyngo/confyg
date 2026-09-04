# Form IR

The tree a host renders, as `confyg-form` produces it today. Canonical for *what a node means*;
the reasoning behind the shape lives in [the design record](../spec/2026-09-03-confyg-design.md)
§3 and is not restated here.

Serialization is stable: every sum type is externally tagged on `kind`, every field is
camelCase, and `web/src/types.ts` mirrors it by hand. `confyg-form/tests/snapshot.rs` pins it.

## Producing one

```rust
confyg_form::compile::project(schema, doc, host) -> Compiled   // Schema + Document
confyg_form::compile::compile(schema, host)                     // = project(schema, None, host)
```

`compile` is the all-**Absent** tree: a form for a file that does not exist yet. Neither call
does I/O, holds state, or mutates anything — an unresolved Schema reference is handed back to
the host as a `SchemaFetchRequest`, never fetched.

## The five node kinds

| `kind` | Carries | Is |
|---|---|---|
| `field` | `widget`, `intended`, `presence`, `meta` | One editable value |
| `group` | `children`, `occupancy`, `toggle` | An object, optionally switchable on and off |
| `repeat` | `items`, `occupancy`, `bounds`, `itemTemplate`, `labelFrom` | A collection |
| `unknown` | `rawPreview` | A key the Schema does not describe — preserved, never a failure |
| `cyclic` | `schemaPtr` | Where a `$ref` cycle made the walk stop |

Every node carries a `path`, and a Path is the only way a host addresses a node.
`confyg_form::search::path_text` renders it (`servers[0].host`); `web/src/types.ts` `pathText`
produces the same string, and `web/` puts it on `data-path`.

`unknown` and `cyclic` are real nodes with real Paths on purpose: a preserved key nobody can
find is a preserved key nobody can fix.

## Presence — a Field's three states

| `kind` | Means | Carries |
|---|---|---|
| `absent` | The key is unwritten; the Schema's `default` applies | `default`, `remarked` |
| `set` | A written, valid literal | `literal` |
| `invalid` | A written literal the Schema rejects — rendered, never discarded | `literal`, `violations` |

An unwritten default is `absent`, never `set` (ADR 0003): "not set" and "set to the same value
as the default" are different facts about a file, and confyg preserves the difference in both
directions. `remarked` is carried from v0.1 but never produced — `Remark` is a v0.2 intent, and
adding the field later would be a breaking IR change for hosts.

An `invalid` literal is shown as authored. Nothing in the pipeline rewrites it on the way to a
screen.

## Occupancy — a container's three states

`absent`, `empty`, `populated`. `empty` and `absent` are different facts too: an empty array
that was written is not a missing array. The distinction is what `AddRepeatItem`'s two lowerings
turn on (see [`intents.md`](intents.md) *Absent-parent lowering*).

## Widget — a closed vocabulary

`text` · `rawText` · `displayOnly` · `radio` · `menu` · `filterableMenu` · `checkboxSet` ·
`tristate` · `stepper` · `slider` · `textarea` · `masked`

A Field carries two: `widget`, which the host must mount, and `intended`, which the Schema asked
for. They differ when the **Host clamp** degraded the control, and both are kept so a host can
say *why* it is showing something else rather than silently substituting. The vocabulary is
closed so an unmapped Widget is a build error in `web/src/widgets/index.ts`, not a blank row.

`filterableMenu` is never mounted in v0.1: the clamp turns it into `menu` and the host renders
the degradation notice.

## Field metadata

`meta` flattens `NodeMeta` — `title`, `description`, `violations`, `locked`, `deprecated` — and
adds `default`, `examples`, `required`, `readOnly`, `writeOnly`, `unit`, `constraints`, `raw`,
and `options`.

Two rules a host depends on:

- **`options` is the only source of choices.** The menu family's `options` arrive in Schema
  order with the label a person reads already resolved. Every other Widget gets an empty list. A
  host must never re-derive choices from `enum`, or `x-confyg.optionLabels` would be honored in
  one host and not the next.
- **`constraints` is guidance, never a gate.** It is the renderable subset of the Schema's
  constraints, for display beside a control. Validation is the validator's, and a violating value
  is written and warned about rather than refused (**Soft constraint**).

A `locked` node carries its `reason` and gets no write affordance in any host, whatever its
Widget. `readOnly` mounts as display only for the same reason.

## Collections

`bounds` is `{ min, max }`, either nullable. `itemTemplate` is a `TemplateRef` — a JSON pointer
into the Schema, not inlined text, so the IR stays small and a fragment is rendered on demand.
`labelFrom` names the entry member whose value titles a card.

**The Form IR counts entries.** `Target.index` counts children, comments included.
`confyg_session::ordinal` is the only place that converts, and
[`upstream.md`](upstream.md) *Index spaces* is canonical for why.

## Validation state

`compile`/`project` also report a `SchemaState`, and `confyg_form::unknown::summary(&root,
&state)` turns it plus every attributed **Violation** into a `Summary`. The state that matters
most is the one where validation could not run at all: one `pattern` the engine cannot compile
disables validation for the whole document (D8), and the form must still project completely. A
host says *validation unavailable* and never *no problems* — `web/src/summary.test.ts` pins
exactly that.

## What v0.1 does not project

Against a real published Schema these are the common ones, each reported as a `Notice` rather
than a silent omission. Driving schemastore's `.eslintrc` produces all four:

| Notice | Effect |
|---|---|
| `form.compile.excluded` — `oneOf` | The node projects as `unknown`; v0.3 implements unions |
| `form.compile.excluded` — `additionalProperties as a schema` | Same; v0.2 implements it |
| `form.compile.allof-conflict` | Conflicting `properties` in an `allOf` merge keep the first |
| `form.degrade.generic` | The Host clamp substituted a control |

So on eslintrc, `extends`, `globals`, `ignorePatterns` and every `rules.*` member render as
preserved-but-unmodelled keys. They are still written back byte-identically; they just have no
control.
