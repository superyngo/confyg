# 0002 — Schema-driven projection and three-state Presence

Date: 2026-09-03

## Decision

confyg projects the **Schema** into the **Form IR** and consults the **Document** as an overlay,
the inverse of confy's document-driven projection. Every **Field** therefore carries a
three-state **Presence** — **Absent**, **Set**, **Invalid** — rather than confy's implicit
node-exists / node-absent pair.

Two rules follow and are binding:

1. **Unset** deletes the key from the **Document**. It never writes the Schema `default`.
2. A boolean **Field** renders as a three-state control (inherited default / `false` / `true`),
   never a bare checkbox.

## Why

confy walks the document to produce its rows, so a key absent from the file does not exist as
far as the UI is concerned. A configurator has the opposite job: its whole value is showing the
settings that *could* be set, with their defaults and documentation, including the ones the file
has never mentioned. That inversion is not a rendering preference — it is what makes
"render a config template from a schema" possible at all, and it is why a **Document** may be
empty, or absent entirely, and still produce a complete form.

Once the Schema drives projection, two states stop being enough. "Key absent" and "key present
and equal to the default" are different facts with different futures: the absent one keeps
inheriting whatever the consuming application decides next release, the written one is pinned
forever. Collapsing them produces bloated config files that silently shadow upstream default
changes — the single most common complaint about generated configuration. A third state is needed
because a real file also contains values that violate the Schema, and by **Soft constraint** those
must be preserved and warned about, never corrected or dropped.

The tri-state boolean is a direct consequence, not a separate stylistic choice: a two-state
checkbox has nowhere to put "inherited", so it necessarily conflates **Absent** with `false`.

## Alternatives rejected

**Reuse confy's row model, adding a "missing" row flag.** Attractive because `ViewRow` and the
DOM reconciler already exist. Rejected because the row model is built around the document as
source of truth — ordering, expansion, and identity all derive from document position, which a
Schema-declared-but-absent field does not have. Retrofitting synthetic rows would leave every
consumer branching on whether a row is real.

**Two states, writing the default on first touch.** Simpler to implement and to render. Rejected
for the shadowing problem above: it makes every generated file a snapshot of one version's
defaults, and it makes "what did I actually configure?" unanswerable from the file.

**Two states plus a separate "is default" annotation on the value.** Rejected as the same thing
with extra steps: the annotation *is* the third state, only unnamed and therefore inconsistently
handled.

## Precedent

The three-state pattern is settled practice in every mature configuration setter, which is part
of why deviating from it would be surprising:

- **VS Code Settings UI** tracks `isConfigured`, shows unconfigured settings' defaults in muted
  text, and implements *Reset Setting* by writing `undefined` — that is, by deleting the key from
  `settings.json`.
- **GNOME dconf-editor** puts a "Use default value" switch on every key; enabling it erases the
  key from the database rather than storing the default.
- **Windows Group Policy (ADMX)** exposes the three states explicitly as *Not Configured* /
  *Enabled* / *Disabled*, and *Not Configured* deletes the registry value.

## Consequences

- **Unset** requires `Mutation::Delete`, and adding a **Repeat group** item or a missing required
  **Field** requires `Mutation::Insert`. confyg drops the user's freedom to restructure the
  document, not the engine's ability to insert and delete; every such write is Schema-gated.
- confy's `web/render.ts` and `ViewRow` cannot be reused. The form renderer is new code.
- The **Form IR** compiler must resolve a Schema `default` for every **Field**, including through
  `$ref` and `allOf`, since **Ghost text**, **Unset**, and **Template** generation all depend on
  it. `confy-core`'s `schema/hints_edit.rs` does not read `default` today and must be extended.
- **Minimal write** becomes a correctness rule, not an optimization: writing a value equal to the
  default would move a **Field** from **Absent** to **Set** and change its future behavior.
