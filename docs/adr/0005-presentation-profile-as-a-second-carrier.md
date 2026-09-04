# 0005 — The Presentation profile as a second carrier

Date: 2026-09-04

[ADR 0004](0004-presentation-layer-model-and-write-neutrality.md) named the presentation layers
and fixed their authority. This ADR decides *where a presentation decision is written*.

It reverses a decision already recorded: [`glossary.md`](../reference/glossary.md)'s **Annotation**
entry lists `UI schema` under `_Avoid_`, naming "the separate-file approach confyg rejected". That
rejection is superseded here, and deliberately kept in its own ADR so that reinstating it later
needs one revert rather than an unpicking of the layer model.

## Decision

**1. One vocabulary, two carriers.** A single closed **Presentation vocabulary** is defined once
and expressible in two places: an `x-confyg` **Annotation** inside the **Schema**, and an optional
sidecar, the **Presentation profile**. A profile's per-node entry and an `x-confyg` object are the
same value — same type, same validation, same documentation.

**2. The Annotation is one object, not a family of keys.** `x-confyg: { affordance, order, unit,
collapsed, demoted, label, help, labelFrom, optionLabels }` — nine optional members, closed for
v0.1. This replaces the design record's bare `x-order` (§4 step 5), which risked colliding with
other tools, and it absorbs §10's hardcoded `label_from` key list. `optionLabels` is included
because a machine-valued `enum` (`"aes-256-gcm"`, `"rr"`) has no standard JSON Schema keyword to
carry a display name, and the `oneOf`-of-`const`-with-`title` workaround would turn a **Field**
into a **Union field** — structural damage to express a label.

**3. Resolution chain.** Every presentation value resolves through, in order:

```
built-in derivation  →  x-confyg Annotation  →  Presentation profile  →  HostProfile clamp  →  user preference
```

The derivation is a total function, so a **Schema** carrying no `x-confyg` and a session with no
profile still produce a complete form; every later stage is a sparse override. The profile
outranks the **Annotation** because the Schema is upstream and the profile belongs to whoever is
deploying confyg — a deployment must be able to override an upstream author's taste in its own
file. The `HostProfile` clamp is last but one because it is physics. User preference is last and
**may only move downward** along a **Degradation ladder**: a user may ask for a stepper instead of
a slider, a plain menu instead of a filterable one, one scrolling page instead of a wizard. It
never upgrades and never changes semantics, which is what keeps it from becoming a fifth author of
the form.

**4. What a profile may carry, and what only it may carry.** The test is scope:

> Does the knob's scope equal exactly one subschema node?

Yes — legal as an `x-confyg` **Annotation**, and equally legal in a profile. No — profile only.
That single rule assigns **Flow** (a step definition references many **Paths**), **Appearance** (a
background image is a property of the application, not of the configuration's meaning), and
**Lexicon** translation tables to the profile without any per-case judgement.

**5. Profile sections and keying.** Four sections: `nodes` (per-node presentation), `flow`,
`lexicon`, `appearance`. `nodes` is keyed by **Schema** pointer (`#/properties/servers/items/properties/host`),
never by **Document** **Path**. **Affordance** is a property of a subschema, not of an instance:
`servers[0].host` and `servers[1].host` rendering differently is a defect, and instance keys would
break the moment the user pressed `+ Add`. This matches C8's schema-path-keyed translation table.

**6. Nothing may be hidden.** The vocabulary has no `hidden`. It has `collapsed` (initially folded)
and `demoted` (sorted to the end of its section). A hidden `required` **Field** with no `default`
produces a **Document** that cannot be made valid from the UI, and gives the user no visible cause
to investigate.

**7. Profile discovery.** Mirrors §6's Schema resolution deliberately, so the two reuse one mental
model: an explicit choice by the user or the command line → a **Profile hint**, `x-confyg.profile`
at the **Schema** root, resolved relative to the Schema → a sibling-file convention, tried against
the **Document**'s name first and the **Schema**'s name second → nothing, meaning pure derivation.
The **Document** is tried first because a deployment always has the config file and often does not
own the Schema, which may be a SchemaStore URL with no local sibling to place a file beside. A
profile may be written in any of the three **Doc formats**; `confy-core`'s loader already reads
all three, so restricting it would be friction with no benefit.

**8. A profile never narrows the Value contract.** No section may add or tighten a constraint. A
deployment wanting a narrower `enum` writes an `allOf` in a Schema — that is a **Value contract**
change and belongs in the file that carries the contract. This is **Write-neutrality** applied to
the carrier.

**9. Unknown profile keys are Notices.** The profile's own JSON Schema, which confyg publishes,
sets `additionalProperties: true`; an unrecognized key raises a **Notice** collected in the
**Violation** summary panel under its own heading, and is otherwise ignored. The **Notice** is not
attached to any **Form node** — it describes the profile, and hanging it on a node would read as a
fault in the user's configuration.

**10. The sidecar is v0.2.** v0.1 ships the layer model, the `x-confyg` vocabulary, the
`HostProfile` clamp, and the built-in derivations; **Flow** and **Appearance** therefore have no
override entry point in v0.1. The carrier's *position* is decided now — the vocabulary is
isomorphic and the chain is fixed — so adding the file later is additive rather than a
redesign.

## Why

**Why the original rejection was right, and why it no longer applies.** A separate UI-schema file
was rejected because a form must render completely from a SchemaStore Schema that will never carry
confyg-specific keywords, and a design whose layout lives in a second file that nobody wrote
produces no form at all. Making every stage of the resolution chain a sparse override of a total
derivation removes that objection entirely: the profile's absence is not a missing input. What
remains of the original concern is a real constraint, retained here — the derivation must stay
total, and no feature may be reachable only through a profile.

**Why annotations alone are not enough.** **Flow** and **Appearance** have nowhere to live. A step
definition spans many nodes, so encoding it as per-node annotations scatters one decision across
the Schema in a form no one can read or verify. A background image has no relationship to any
subschema at all. Keeping annotations as the only carrier does not simplify the design; it leaves
two of the eight layers permanently unspecified.

**Why not the profile alone.** It would strip Schema authors of the ability to describe their own
data — the party who knows that a field is a byte count, or that its label should read "Listen
address", is the Schema author — and it contradicts the existing `labelFrom` and ordering design.

**Why the Annotation is a single object.** It is what makes "one vocabulary, two carriers" real
rather than aspirational: the annotation's value and a profile `nodes` entry are literally the same
struct, validated by the same schema and documented once. It also pollutes the Schema with one key
instead of nine, and extending the vocabulary later needs no new top-level key.

**Why the profile outranks the Schema.** Precedence follows ownership. The Schema is upstream and
shared; the profile is the deployment's own file. Inverting this would mean a deployment could not
adapt a vendored Schema without forking it.

## Alternatives rejected

**Keep annotations only (the status quo).** Rejected above: **Flow** and **Appearance** stay
unspecifiable.

**Profile only, dropping `x-confyg`.** Rejected above: removes the Schema author's voice and
conflicts with existing design.

**Flat annotation keys (`x-confyg-affordance`, `x-confyg-order`, …).** Marginally more readable in
isolation. Rejected because the two carriers stop being the same value, which is the entire
mechanism by which one vocabulary serves both.

**Keying `nodes` by Document Path, or by both.** Rejected: **Affordance** is a subschema property,
and instance keys invalidate themselves as soon as a **Repeat group** grows.

**`hidden: true`, optionally suppressed for required fields without a default.** The conditional
form looked safe. Rejected because the exception is invisible in the profile: the author writes
`hidden`, it silently does not apply, and the two behaviours are indistinguishable at the point of
authorship. `collapsed` serves the real need — large Schemas need advanced sections folded — with
no unreachable states.

**`additionalProperties: false` on the profile schema.** Rejected because it makes an older confyg
refuse a newer profile outright. Closing the vocabulary (Decision 2) exists to give forward
compatibility; enforcing closure at read time would spend it.

**Shipping the sidecar in v0.1.** Rejected on evidence, not effort: the shape a profile should take
is best judged against two or more real deployments, and v0.1 has none. The reversal cost is low
because the chain and vocabulary are fixed now.

## Consequences

- `glossary.md`'s **Annotation** entry loses `UI schema` from its `_Avoid_` list and gains a
  cross-reference to **Presentation profile**. New glossary entries: **Presentation vocabulary**,
  **Presentation profile**, **Profile hint**, **Affordance**, **Flow**, **Partition**,
  **Traversal**, **Lexicon**, **Appearance token**, **Host capability**, **Degradation ladder**,
  **Write-neutrality**, **Conduct**, **Form search**, **Option filter**.
- The term *meta schema* is not used for the sidecar anywhere in this repo. In JSON Schema it
  already means the schema describing schemas; the profile's own published schema is "the
  profile's schema".
- §4 step 5's `x-order` becomes `x-confyg.order`; §10's hardcoded `label_from` key list becomes
  `x-confyg.labelFrom` with the current list as the derivation default.
- confyg publishes and versions a JSON Schema for the **Presentation profile**, which also gives
  profile authors editor completion via an ordinary **Schema hint**.
- §9's tiers grow the presentation work: v0.1 the layer model, the vocabulary, the clamp, `scroll`
  and `sections`, **Form search**; v0.2 the sidecar, `tabs`, the filterable-menu combobox, and the
  **Lexicon** translation tables; v0.3 `wizard` and the TUI `HostProfile`.
- **Preset value sets** (C10) are not a profile section. They write bytes, and although they do so
  only through explicit **Setter intents**, admitting them would make the profile's write-neutral
  reading of §8 conditional. They get their own carrier when they are designed.
