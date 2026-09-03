# 0003 — Form IR shape: Presence scope, collection unification, and default equivalence

Date: 2026-09-04

Extends [ADR 0002](0002-schema-driven-projection-and-three-state-presence.md), which established
**Schema-driven projection** and three-state **Presence**. Three consequences of that decision
were left underspecified; grilling the design record surfaced them together, and they are decided
together because each constrains the other two.

## Decision

**1. Presence stays a Field concept. Containers get their own two dimensions.**

`Presence` remains exactly `Absent` / `Set` / `Invalid` and appears only on a **Field**. Every
**Form node** — **Field** included — carries `violations: Vec<Violation>` in its `NodeMeta`, and
every container additionally carries an **Occupancy**: `Absent` / `Empty` / `Populated`.

**2. A scalar array is a Repeat group, not a Field.**

**Field** means one settable value, full stop. `array` of scalars and `array` + `items.enum`
compile to a **Repeat group** whose items are **Fields**; the multi-choice presentation is a
**Widget** hint on that group.

**3. Setting a value equal to the Schema default is Unset.**

`SetValue` with a value equal to the effective `default` deletes the key. The default is rendered
as the inherited option *within* the control — the same affordance as **Unset** — so the two are
one user action rather than two actions with one outcome.

## Why

**Presence scope.** ADR 0002 gave every **Field** a three-state value status, but the **Form IR**
put `presence` only on `Field` and overlaid violations only onto leaf **Paths**. Containers have
both a state and violations, and neither had anywhere to live:

- `servers = []` and no `servers` key at all are different facts under `minItems: 1`, exactly as
  "absent" and "written equal to the default" are different facts for a scalar. ADR 0002's whole
  argument is that collapsing two such states produces silently wrong behaviour; the same
  argument applies one level up.
- The validator reports `minItems`, `maxItems`, `uniqueItems`, `required`, `minProperties`, and
  `additionalProperties` against the **container's** path, never a child's. With violations
  attached only to leaves, those failures had no home, and the violation summary's jump-to-field
  had no target.

Widening `Presence` to containers was rejected: `Set { literal: String }` is meaningless for a
table, and an `Invalid` group would conflate "this group's own constraint failed" with "something
inside it failed". Two orthogonal dimensions — occupancy and violations — say what is true without
overloading a term whose precision the glossary protects.

**Collection unification.** The design classified `array` of scalars as "a **Field** with a list
**Widget**", which contradicts the glossary's **Field** ("one settable scalar value") and broke
three things at once: **Presence** would have to hold *n* values in one `literal`; a per-item
**Violation** at `tags[2]` had no node to attach to; and `RemoveRepeatItem` / `MoveRepeatItem`
were gated to **Repeat groups**, leaving scalar lists with no way to remove or reorder an entry —
while the design simultaneously promised order-significant lists.

A scalar array *is* a bounded, index-addressed, add/remove/reorderable collection. That is
**Repeat group**'s definition minus the word "objects". Unifying them deletes an IR variant,
inherits the intents for free, and keeps **Field** meaning one value — which is what makes
**Presence**, **Ghost text**, and **Unset** coherent.

**Default equivalence.** **Minimal write** was written for **Template** generation, and `SetValue`
had no corresponding gate, so the same file could mean two different things depending on whether a
value arrived by generation or by typing. Of the three available outcomes, writing the default
pins it forever against upstream change — the exact failure ADR 0002 exists to prevent — and
marking it redundant is that failure with a label on it. Deleting the key is the only outcome that
keeps **Minimal write** an invariant of the **Document** rather than a rule that holds at one
moment.

The glossary warns that **Unset** must not read as "write the default". Rendering the default as
the inherited option inside the control resolves that: the user is not pressing *set* and getting
*unset*, they are selecting "inherit", which is a distinct, visible, honest choice. The tri-state
boolean ADR 0002 mandates is already precisely this control, generalized.

## Alternatives rejected

**Presence on every node.** Uniform and briefly attractive. Rejected because `Set { literal }` has
no meaning for a container, and because it would make "the group is invalid" and "a child of the
group is invalid" indistinguishable at the node the user is looking at.

**Violations in a flat side-table keyed by Path, joined by the host.** Keeps `NodeMeta` small.
Rejected because every host would reimplement the join, and `PointerMap::resolve` already walks
up to the nearest present ancestor, so the join is subtler than it looks.

**Widening Field to "one settable value".** One glossary edit instead of an IR change. Rejected
because it makes per-item violations and per-item removal unrepresentable, which is where the real
cost lands.

**A ninth Form node for scalar lists.** Honest, but it duplicates **Repeat group**'s bounds,
intents, and rendering to express "the items happen to be scalars" — a property of the item
schema, not of the collection.

**Letting `SetValue` pin a default.** Simplest to implement. Rejected above.

## Consequences

- `NodeMeta` grows `violations`; containers grow `occupancy`. The compiler's overlay step visits
  every node, not only leaves, and must attribute container violations by exact path — noting that
  an unresolvable pointer falls back to the root, and that `additionalProperties: false` names its
  offending keys only in the message.
- **Repeat group**'s glossary entry loses "of objects"; `Repeat.items` may hold **Fields**. A6's
  checkbox set and B6's scalar list are **Widget** hints, not node kinds.
- The **Presence** matrix in verification gains the occupancy transitions: absent → empty →
  populated, and the reverse via removal.
- Every control that can express its Schema `default` must render an inherited option; a control
  that cannot (free text with no default) keeps a separate **Unset** affordance.
- A user cannot pin a value equal to the current default. If a Schema's default later changes,
  their file follows it. That is the intended behaviour, and it is worth stating in the UI when the
  inherited option is selected.
