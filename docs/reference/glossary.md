# Glossary

The canonical vocabulary. Code identifiers, UI strings, commit messages, and every other
document in this repo use these terms. Introducing a new term means adding its entry in the
same commit.

Terms marked *(inherited)* come from [confy](https://github.com/superyngo/confy)'s
`docs/reference/CONTEXT.md` and keep exactly the same meaning here; confyg depends on
`confy-core` and must not fork their definitions.

## The product

**Setter**:
confyg's operating concept: a surface that fills in a **Document** by choosing values the
**Schema** already declares. A Setter never offers a structural operation the Schema does not
imply — no arbitrary insert/delete/move, no notation switching, no type conversion.
_Avoid_: Editor (confy is the editor; the distinction is the whole point), form builder.

**Configurator**:
Synonym for **Setter**, used only in prose aimed at users. Never a type or module name.
_Avoid_: —

## Inherited document model

**Node** *(inherited)*:
Any single element in the config tree of a **Document**.
_Avoid_: Entry, item.

**Path** *(inherited)*:
A **Node**'s address from the root, `Vec<Seg>` where a `Seg` is a key or an array index. The
join key between the **Document** and the **Form IR**.
_Avoid_: Pointer (that is the RFC 6901 form), address.

**Document**:
The config file under configuration, held as a lossless CST by `confy-core` so untouched
regions round-trip byte-identically. One of three **Doc formats**: TOML, JSON(C), YAML.
_Avoid_: File, buffer, config (ambiguous — a config is the *data*, a Document is the *file*).

**Doc format** *(inherited)*:
TOML, JSON(C), or YAML — the concrete syntax a **Document** is written in.
_Avoid_: Language, filetype.

**Schema hint** *(inherited)*:
The in-**Document** pointer to a JSON Schema, recognized per **Doc format**: a JSON root
`"$schema"` member, a YAML leading `# yaml-language-server: $schema=…` modeline, or a TOML
first-line `#:schema …` comment. Two of the three are lexically comments, which is why a
comment-preserving CST is a hard requirement.
_Avoid_: `$schema` (that is only the JSON spelling), schema comment.

**Violation** *(inherited)*:
A single JSON Schema constraint failure reported against a **Path**. Purely informational.
_Avoid_: Error, validation failure.

**Soft constraint** *(inherited)*:
The governing rule for Schema support: a Schema's rules surface only as **Violations** — never
a rejected input, never a blocked save. A wrong value warns.
_Avoid_: Hard constraint, validation gate.

## Schema-driven projection

**Schema**:
The compiled JSON Schema that drives the form. Loaded from a **Schema hint**, a CLI argument, or
a user choice. 2020-12 is the working vocabulary; a Draft 7 Schema is validated under Draft 7
semantics and normalized to 2020-12 spellings before compilation. A Schema has two independent
states — *projected* and *validatable* — because one uncompilable `pattern` costs the whole
**Document** its validation without affecting its form.
_Avoid_: Spec, model, definition.

**Schema-driven projection**:
confyg's projection direction: the **Schema** is walked to produce the **Form IR**, and the
**Document** is consulted as an overlay to fill in **Presence**. The inverse of confy, which
walks the Document and treats the Schema as a decoration. Its consequence is that a field the
Schema declares is visible even when the Document has no such key, so a Document may be empty
or absent entirely.
_Avoid_: Schema-first (means the narrower case of starting with no Document), form generation.

**Form IR**:
The ordered tree of **Form nodes** compiled from a **Schema** and a **Document**. The single
value every host renders; it carries no host or widget-toolkit concepts.
_Avoid_: View model, form tree, AST.

**Form node**:
One member of the **Form IR**. Exactly one of: **Field**, **Group**, **Repeat group**, **Map
group**, **Variant group**, **Union field**, **Tuple group**, **Unknown group**, **Cyclic stub**.
Every Form node carries its own **Violations** — the validator reports a container's failures
against the container's **Path**, never a child's.
_Avoid_: Element, control, row.

**Field**:
A **Form node** that holds one settable value. Carries a **Widget**, a **Presence**, and its
display metadata (title, description, default, unit, deprecation). One value, not one *scalar*: a
**Union field** holds one value of a chosen type, while a list of values is a **Repeat group**.
_Avoid_: Setting, option, leaf, property.

**Widget**:
The input affordance a **Field** resolves to, decided by the Schema keywords governing it
(`enum` → menu, `boolean` → tri-state toggle, bounded number → stepper, and so on). A **Form
IR** concept, not a DOM or ratatui concept; each host maps a Widget to its own control.
_Avoid_: Control, input type, renderer.

**Group**:
A **Form node** for a Schema `object` with declared `properties` — a fixed set of named
children. An optional Group (one not in `required`) carries a toggle that writes or removes the
whole section.
_Avoid_: Section, table, fieldset, object.

**Repeat group**:
A **Form node** for a Schema `array` — the `+ add another server` case. Items are
index-addressed; add and remove are gated by `minItems` / `maxItems`. Items are **Groups** when
the item schema is an `object` and **Fields** when it is a scalar, so a scalar list is a Repeat
group too rather than a multi-valued **Field**.
_Avoid_: Array of tables (that is the TOML **Emission style**, not the concept), list, collection.

**Map group**:
A **Form node** for a Schema `object` whose children are user-named — `additionalProperties` or
`patternProperties`. The only **Form node** whose keys the user may author or rename; keys are
checked against `propertyNames` / the matching pattern and against collision.
_Avoid_: Dictionary, free-form object, additional properties.

**Variant group**:
A **Form node** for a Schema `oneOf` / `anyOf` of objects: a mandatory choice among named
alternatives. Selecting a variant never migrates data between alternatives.
_Avoid_: Union, polymorphic field, discriminated union.

**Tuple group**:
A **Form node** for Schema `prefixItems` — fixed positions with a per-position Schema.
Reordering and removal are unavailable.
_Avoid_: Fixed array, positional array.

**Union field**:
A **Form node** for a Schema `anyOf` of scalars: the user chooses a type, then a value. A
**Field** rather than a **Variant group** because it holds exactly one value; switching type
clears to **Absent** rather than coercing the previous literal.
_Avoid_: Scalar union, type union, polymorphic field.

**Unknown group**:
The **Form node** collecting **Unknown keys**. Rendered last, collapsed, read-only apart from
removal.
_Avoid_: Extra keys, custom section, orphans.

**Unknown key**:
A key present in the **Document** that the **Schema** does not describe. Always preserved
byte-for-byte and surfaced with a **Notice**, because a hand-written config file that predates
the Schema must still be openable. Never rewritten, never dropped, and never given a fabricated
**Violation**: under `additionalProperties: true` an extra key breaks no rule.
_Avoid_: Invalid key, stray key, unrecognized setting.

**Cyclic stub**:
The placeholder a recursive `$ref` compiles to. Expanding it compiles one more level on demand,
which is what keeps a self-referential Schema from recursing forever.
_Avoid_: Lazy node, recursion guard.

## Value state

**Presence**:
A **Field**'s three-state value status, and the reason confyg cannot reuse confy's
node-exists/node-absent pair. Exactly one of **Absent**, **Set**, or **Invalid**. A **Field**
concept only; a container's equivalent is **Occupancy**.
_Avoid_: State, status, dirty.

**Absent**:
The **Presence** of a **Field** whose key is not in the **Document**: the value is inherited
from whatever default the consuming application applies. Displayed as the Schema `default` in
**Ghost text**. Carries the commented-out prior value when the key was disabled rather than
deleted (v0.2).
_Avoid_: Empty, null, unset (that is the *action*, not the state), default.

**Set**:
The **Presence** of a **Field** whose key is written in the **Document** and satisfies the
**Schema**.
_Avoid_: Configured, dirty, explicit.

**Invalid**:
The **Presence** of a **Field** whose key is written in the **Document** but carries one or more
**Violations**. The authored text is preserved exactly; only a warning is added.
_Avoid_: Error, broken, rejected.

**Ghost text**:
The muted rendering of a Schema `default` on an **Absent** **Field**, showing the inherited
value without claiming it is written.
_Avoid_: Placeholder, hint, watermark.

**Unset**:
The action that returns a **Set** or **Invalid** **Field** to **Absent** by deleting its key
from the **Document** — never by writing the default value. Unavailable on a `required` Field.
_Avoid_: Reset (ambiguous — reads as "write the default"), clear, revert, delete.

**Occupancy**:
A container's three-state fill status: **Absent** (no key), *Empty* (`servers = []`), or
*Populated*. A container's counterpart to **Presence**, kept separate because `Set { literal }`
is meaningless for a table and because `minItems` makes empty and absent different facts.
_Avoid_: Presence (that is the **Field** term), count, size.

**Locked**:
The marker on a **Form node** whose text confyg will not write: a YAML alias, or a value
inherited through a merge key. The resolved value is rendered with an explanatory notice and no
write affordance, because rewriting an alias would either destroy the reference or silently
change every other site that shares the anchor.
_Avoid_: Read-only (that is the Schema keyword), frozen, immutable.

**Raw literal fallback**:
The rule that a **Field** whose literal does not parse as its **Widget**'s type renders as raw
text carrying its **Violation**, and offers the typed control again once the literal parses. Also
enterable deliberately, which is what allows an interpolated `${VAR}` in a field the Schema types
as an integer. The mechanism that makes **Soft constraint** renderable.
_Avoid_: Text mode, escape hatch, override.

**Notice**:
An informational message attached to a **Form node** that the **Schema** did not produce — an
**Unknown key**, a **Locked** node, a compile diagnostic. Distinct from a **Violation**, which is
always something the validator actually reported.
_Avoid_: Warning, violation, hint, diagnostic (that is the compiler's internal term).

## Writing

**Setter intent**:
One member of confyg's closed set of **Document** mutations (`SetValue`, `Unset`,
`AddRepeatItem`, `RemoveRepeatItem`, `MoveRepeatItem`, `AddMapEntry`, `RenameMapKey`,
`RemoveMapEntry`, `SelectVariant`, `SelectUnionType`, `ToggleGroup`, `GenerateTemplate`). Every
intent is Schema-gated: the **Schema** decides whether it is offered and what it may write.
Session-level operations are **Session commands**, not intents.
_Avoid_: Command, action, mutation (a Mutation is the `confy-core` primitive an intent lowers to).

**Session command**:
An operation on the session rather than on the **Document**'s content, and therefore not
Schema-gated: `Open`, `Save`, `ConvertFormat`, `LoadSchema`, `Undo`, `Redo`.
_Avoid_: Setter intent, action, host call.

**Template**:
A **Document** generated from a **Schema** alone, with no prior file: required **Fields** and
declared defaults written out, plus the **Schema hint** for the target **Doc format**.
_Avoid_: Skeleton, scaffold, boilerplate, starter.

**Template strategy**:
Which **Fields** a **Template** writes: `required-only`, `with-defaults`, or `all-fields`.
_Avoid_: Template level, mode, preset.

**Minimal write**:
The rule that a value equal to its Schema `default` is not written to the **Document**. It keeps
generated files small and, more importantly, lets an upstream default change reach the user
instead of being shadowed by a redundant line.
_Avoid_: Sparse write, prune, compact.

**Emission style**:
The per-**Doc format** choice of concrete syntax when confyg writes a **Form node** that has no
prior text — a **Repeat group** as TOML `[[servers]]` rather than `servers = [{…}]`, a **Group**
as a scope table, a YAML sequence as a block sequence. Fixed policy, never a user choice; this
is precisely the notation freedom confyg drops relative to confy. For a TOML **Repeat group** the
style is expressed by which `Target` the insert addresses, not by the fragment's text.
_Avoid_: Kind, notation, format (all three already mean something else), style switch.

**Table ordering rule**:
TOML's requirement that a table's scalar members precede its sub-tables. Any **Setter intent**
that inserts into a TOML **Document** must place the new text accordingly or the file's meaning
changes.
_Avoid_: TOML quirk, ordering constraint.

**Absent-parent lowering**:
The rule that every additive **Setter intent** has a second form for the case where the
container does not exist yet: the first member of a collection, the first entry of a **Map
group**, and the enabling of an optional **Group** each create the container as well as the
member, and therefore lower to a different `Insert` than the second member does.
_Avoid_: Bootstrap, create-on-write, first insert.

**Order divergence**:
The rule that where a **Doc format**'s syntax and the **Schema**'s declared order conflict —
TOML's **Table ordering rule** against a Schema declaring a scalar after an object — legality
wins. The **Form IR**'s order is authoritative for the UI; the **Document**'s order is whatever
the format permits, and confyg never reorders existing text to chase the Schema.
_Avoid_: Ordering mismatch, sort conflict.

**Comment policy**:
Whether a **Document** may carry comments confyg wrote. Derived per Document, never chosen:
TOML and YAML always may; `.jsonc` / `.json5` may; `.json` may not, unless it arrived with
comments already, which shows its consumer tolerates them. Only **Template** generation emits
comments; an existing Document's comments are never touched.
_Avoid_: JSONC mode, comment support, format flag.

## Annotations

**Annotation**:
An optional `x-`-prefixed Schema keyword confyg understands. There is exactly one:
`x-confyg`, whose value is a **Presentation vocabulary** entry, plus `x-confyg.profile` as a
**Profile hint** at the Schema root. Always optional: a Schema carrying none must still render a
complete, usable form, so any Schema from SchemaStore works unmodified. The **Presentation
profile** is the second carrier of the same vocabulary, not a competing mechanism (ADR 0005).
_Avoid_: Extension, vendor keyword, hint (a **Schema hint** is a different thing), meta schema
(in JSON Schema that already means the schema describing schemas).

## Presentation layers

**Value contract**:
The layer of JSON Schema keywords that decide what a value may be — type, bounds, `enum`,
`pattern`, `required`, `default`, and the conditionals. One of the two layers permitted to change
what is written to the **Document**.
_Avoid_: Validation layer, data layer, model.

**Affordance**:
The layer deciding which **Widget** a **Field** resolves to. Derived from the **Value contract**,
overridable per node. A property of a subschema, never of an instance: two items of one **Repeat
group** always render alike.
_Avoid_: Widget layer (a Widget is the unit, Affordance is the layer), presentation, control.

**Flow**:
The layer deciding how the **Form IR** is split across screens and how the user moves between
them — exactly **Partition** and **Traversal**. Never decides which nodes *exist*; that is the
**Value contract**'s conditionals.
_Avoid_: Navigation, routing, layout, wizard.

**Partition**:
How the **Form IR** is cut into screens: `scroll`, `sections`, `tabs`, or `wizard`. A closed set.
_Avoid_: Layout, view mode, pagination.

**Traversal**:
How the user moves between **Partition** units. Free by default. May skip a step whose nodes do
not exist; may never withhold forward movement because a value is missing or violates the
**Schema** — that would be the **Soft constraint** the design forbids, in its most obtrusive form.
_Avoid_: Navigation, stepper (that is a numeric **Widget**), gating.

**Lexicon**:
The layer holding every string a form displays. Two separated kinds: chassis strings, keyed flat
under `form.*` and versioned with confyg, and Schema-content strings (`title`, `description`),
keyed by Schema pointer and versioned with the Schema.
_Avoid_: i18n (that is the mechanism, not the layer), copy, labels, strings.

**Appearance**:
The layer holding theme, density, and decorative media, expressed only as **Appearance tokens**.
The clearest case of a layer that cannot live in a **Schema**: a background image has no
relationship to any subschema, so it is carried by a **Presentation profile** and by user
preference alone.
_Avoid_: Theme (a theme is one set of token values), styling, skin.

**Appearance token**:
One semantic role in confyg's OKLCH token set — `--ghost`, `--violation`, `--locked`, and the
rest. A role, never a colour: each host maps a token to its own medium, and a token a host cannot
express degrades to nothing. confyg extends confy's chrome roles and does not inherit its
data-type colours.
_Avoid_: Colour, variable, palette, theme (a theme is a set of token values).

**Conduct**:
Keyboard, focus, selection, and scrolling behaviour. A **closed** layer: governed by
`wens-dev-principles ui`, with no **Annotation**, profile, or preference entry point, because it
is a property of the host platform rather than of the configuration being edited.
_Avoid_: Interaction, UX, bindings.

**Write-neutrality**:
The invariant that no layer above the **Value contract** and **Emission style** may change the
bytes written to the **Document**. Presentation decides what the user sees and how they express an
intent; it never decides what that intent writes. Verified as a property, not reviewed as a
guideline: identical inputs under any presentation must emit identical bytes.
_Avoid_: Separation of concerns, read-only, purity.

**Presentation vocabulary**:
The closed set of nine optional members — `affordance`, `order`, `unit`, `collapsed`, `demoted`,
`label`, `help`, `labelFrom`, `optionLabels` — that expresses a per-node presentation decision.
One vocabulary with two carriers: an `x-confyg` **Annotation** and a **Presentation profile**
entry are the same value. Contains no `hidden`, because a hidden `required` **Field** with no
`default` yields a **Document** that cannot be made valid from the UI.
_Avoid_: UI schema, widget spec, hints.

**Presentation profile**:
The optional sidecar carrying **Presentation vocabulary** entries plus the parts that cannot scope
to a single subschema: `flow`, `lexicon`, `appearance`. Four sections; `nodes` is keyed by Schema
pointer. Outranks an **Annotation**, because the Schema is upstream and the profile belongs to
whoever deploys confyg. May never add or tighten a constraint (**Write-neutrality**). Unknown keys
are **Notices**. v0.2.
_Avoid_: Meta schema (already taken by JSON Schema), UI schema, config (a config is the data),
theme file.

**Profile hint**:
`x-confyg.profile` at a **Schema**'s root, pointing at a **Presentation profile**. The presentation
counterpart of a **Schema hint**, and resolved by a deliberately parallel order: explicit choice →
Profile hint → sibling-file convention, **Document** name before **Schema** name → nothing.
_Avoid_: Profile ref, theme link.

**Host capability**:
What a host can actually render, declared once at session start as a `HostProfile` — pure data, a
flat set of facts plus a density, never trait methods. Pure data because it must serialize into a
compiler snapshot, so a host's degraded form is an assertion rather than a manual check.
_Avoid_: Feature flags, environment, platform.

**Degradation ladder**:
A **Widget**'s ordered fallback chain, terminating in a control every host has. Walked by
`confyg-form` at compile time, never by a host, so the Web and TUI hosts cannot diverge. The
**Form IR** keeps both the clamped **Widget** and the intended one, so a substitution can be
explained. A Widget with no chain to a universal control is not admissible.
_Avoid_: Fallback, polyfill, graceful degradation.

**Form search**:
Fuzzy matching over **Form node** titles, descriptions, and **Paths**, to locate a **Field** in a
large form. A pure function in `confyg-form`, because a hit must be able to move the **Partition**
to the section containing it. Distinct from an **Option filter**.
_Avoid_: Filter (that is confy's document-filtering term), find, query.

**Option filter**:
The type-to-filter affordance *inside* one **Widget**, for an `enum` past the count at which a
plain menu stops being usable. Shares no component and no term with **Form search**.
_Avoid_: Search, autocomplete, combobox (that is the control implementing it).
