# confyg presentation layers
Status: Draft

The design record for how a **Form IR** becomes something a person looks at: which layer decides
what, what vocabulary carries a presentation decision, how a **Widget** degrades on a host that
cannot render it, how a form is partitioned into screens, and where its strings live.

Vocabulary is fixed by [`../reference/glossary.md`](../reference/glossary.md). The two decisions
this record rests on are
[ADR 0004](../adr/0004-presentation-layer-model-and-write-neutrality.md) (the layer model and
**Write-neutrality**) and
[ADR 0005](../adr/0005-presentation-profile-as-a-second-carrier.md) (one **Presentation
vocabulary**, two carriers). It extends
[`2026-09-03-confyg-design.md`](2026-09-03-confyg-design.md), which owns the **Form IR**, the
**Setter intents**, the pattern catalogue, and the write policy; sections there are cited as
"design §n" and are not restated here.

---

## 1. The layers

| # | Layer | Governs | Authority | Carrier |
|---|---|---|---|---|
| 1 | **Value contract** | type, bounds, `enum`, `pattern`, `required`, `default`, conditionals | Schema author | the **Schema** |
| 2 | Structure (**Form IR**) | **Form node** kind, nesting, order | derived | — |
| 3 | **Affordance** | which **Widget** a **Field** resolves to | derived, overridable | `x-confyg`, profile `nodes` |
| 4 | **Flow** | **Partition** and **Traversal** | overridable | profile `flow` |
| 5 | **Lexicon** | labels, descriptions, help, units, translations | overridable | `x-confyg`, profile `lexicon`, translation tables |
| 6 | **Appearance** | **Appearance tokens**, theme, density | overridable | profile `appearance`, user preference |
| 7 | **Emission style** | concrete syntax for new text | **closed** | — |
| 8 | **Conduct** | keyboard, focus, selection, scrolling | **closed** | — |

Layers 7 and 8 have no override entry point at all; opening one requires an ADR superseding
ADR 0004. Layer 7's policy is design §8; layer 8's is `wens-dev-principles ui`.

**Write-neutrality** is the invariant that makes the split meaningful: only layers 1 and 7 may
change the bytes written to the **Document**. Its test is §8 here and verification item 6 in
design §11.

### Resolution chain

```
built-in derivation → x-confyg Annotation → Presentation profile → HostProfile clamp → user preference
```

The derivation is a **total function**: a **Schema** with no `x-confyg` and a session with no
profile produce a complete, usable form. Every later stage is a sparse override. No feature may be
reachable only through a profile.

**Derivation thresholds are constants; derivation results are overridable.** The option count at
which a menu becomes filterable is not a setting (ADR 0004 decision 3). A deployment that wants a
different outcome names the node.

User preference may only move **downward** along a **Degradation ladder** — stepper instead of
slider, plain menu instead of filterable, one scrolling page instead of a wizard. It never
upgrades and never changes semantics.

## 2. The Presentation vocabulary

Nine members, closed for v0.1, all optional. The same value appears as the `x-confyg` **Annotation**
inside a subschema and as a profile `nodes` entry — one type, one schema, one documentation table.

| Member | Type | Meaning | Derivation default |
|---|---|---|---|
| `affordance` | **Widget** name | overrides the resolved **Widget** | §3's precedence |
| `order` | integer | position among siblings | Schema `properties` order (design §4 step 5) |
| `unit` | string | unit badge beside a numeric control | none |
| `collapsed` | bool | section starts folded | `false` |
| `demoted` | bool | sorted to the end of its section | `false` |
| `label` | string | replaces `title` | Schema `title`, else the key |
| `help` | string | replaces `description` | Schema `description` |
| `labelFrom` | string | **Repeat group** card title source key | first present of `name`, `id`, `title`, `host`, else `<title> #<n>` |
| `optionLabels` | object | `enum` value → display string | the raw value |

Notes:

- `x-confyg.order` replaces the design record's bare `x-order`, which risked colliding with other
  tools. `x-confyg.labelFrom` replaces design §10's hardcoded key list, which becomes the
  derivation default above.
- `optionLabels` exists because a machine-valued `enum` has no standard keyword carrying a display
  name, and the `oneOf`-of-`const`-with-`title` workaround would reclassify the **Field** as a
  **Union field** (ADR 0005 decision 2).
- `x-confyg.profile` is a **Profile hint** at the **Schema** root only, not a member of this
  vocabulary.
- There is no `hidden`. `collapsed` and `demoted` cover the real need — large Schemas need advanced
  sections folded — without producing a **Document** that cannot be made valid from the UI.
- No member may add or tighten a constraint. Narrowing an `enum` is an `allOf` in a **Schema**.

## 3. Affordance

**Widget** resolution, unchanged from design §3 in its precedence and extended with the override
and the clamp:

1. `const` / `readOnly` → display-only
2. **Raw literal fallback** active → raw text
3. `x-confyg.affordance`, then profile `nodes[…].affordance`
4. `enum` → menu family (§3.1)
5. `format` → the matching specialized control
6. `type` → the primitive control

then the `HostProfile` clamp (§4), which may substitute but never re-resolve.

The **Form IR** carries both: `widget` is what the host renders, `intended` is what the derivation
chose. They differ only when the clamp fired, and the difference is what lets the form explain
itself (§6).

### 3.1 The menu family and the Option filter

An **Option filter** is the type-to-filter affordance *inside* one **Widget**. It is not
**Form search** (§5.3), and the two share no component and no term.

| `enum` option count | **Widget** |
|---|---|
| ≤ 4 | `radio` |
| 5 – 12 | `menu` |
| > 12 | `filterable-menu` |

12 is a constant. A node needing a different outcome writes `affordance`. A 400-option `enum`
(timezone, locale, colour name) is unusable without an **Option filter**, which is the whole
reason the third row exists.

Upstream has no combobox — only native `<select>` (`confy/web/render.ts:82`). Per §4,
`filterable-menu` therefore clamps to `menu` on every host in v0.1 and is implemented in v0.2.
This is a legitimate state precisely because the **Degradation ladder** exists.

## 4. Host capability and Degradation ladders

A host hands over one `HostProfile` at session start: a flat set of capability facts plus a
density. It is pure data, so it serializes into a compiler snapshot and "what does this form look
like on the TUI" becomes an assertion rather than a manual check.

```rust
struct HostProfile {
    filterable_menu: bool,   // custom combobox available
    slider: bool,            // continuous drag control is meaningful
    swatch: bool,            // colour rendering available
    native_picker: bool,     // file / date pickers provided by the platform
    masking: bool,           // characters can be visually hidden
    background_media: bool,  // decorative imagery renderable
    pointer: Pointer,        // Fine | Coarse | None
    density: Density,        // Compact | Comfortable | Touch
}
```

Every **Widget** declares a ladder terminating in a control every host has. `confyg-form` walks it
at compile time (ADR 0004 decision 6); no host implements the walk, which is what keeps the Web
and TUI hosts from diverging.

| **Widget** | Ladder | Terminal |
|---|---|---|
| `filterable-menu` | → `menu` → `radio` | `radio` |
| `menu` | → `radio` | `radio` |
| `radio` | — | `radio` |
| `checkbox-set` | → `radio` per item | `radio` |
| `tri-state` | — | `tri-state` |
| `slider` | → `stepper` → `text` | `text` |
| `stepper` | → `text` | `text` |
| `masked` | → `text` (with an explicit "not masked here" **Notice**) | `text` |
| `textarea` | → `text` | `text` |
| `date-picker`, `time-picker`, `duration` | → `text` | `text` |
| `path`, `file` | → `text` | `text` |
| `segmented` (IP) | → `text` | `text` |
| `swatch` | → `text` | `text` |
| `monospace` (regex) | → `text` | `text` |
| `display-only` | — | `display-only` |
| `raw` | — | `raw` |

A **Widget** with no chain to a universally available control is not admissible (ADR 0004).
`masked` degrading to plain `text` must announce itself: silently revealing a `writeOnly` value
would be a security surprise, and the **Notice** mechanism already exists for facts the **Schema**
did not produce.

## 5. Flow

Three concepts, of which two are **Flow**:

| Concept | Definition | Layer |
|---|---|---|
| **Partition** | how the **Form IR** is cut into screens | **Flow** |
| **Traversal** | how the user moves between them | **Flow** |
| data-driven structure | `if`/`then`/`else`, `dependentSchemas` deciding which nodes *exist* | **Value contract** (design §4 step 3) |

The first two choose among nodes that exist; the third decides which exist. They are never merged.

### 5.1 Partition

Closed set of four, so a profile written today stays readable by a later confyg:

| Value | Layout | Tier |
|---|---|---|
| `scroll` | one page, sections are headings | v0.1 |
| `sections` | section list beside one section's fields (master-detail) | v0.1 |
| `tabs` | section list as a tab strip | v0.2 |
| `wizard` | ordered steps | v0.3 |

`tabs` and `wizard` clamp to `sections` until implemented. `accordion` is deliberately not a
**Partition**: it is `collapsed` under `scroll`, and listing it would give two routes to one
outcome.

Sections are the root's depth-1 **Group** children — the Schema author's own grouping, which
design §4 step 5 already treats as more meaningful than a required-first sort. **When fewer than
three such Groups exist, `sections` falls back to `scroll`**: a two-pane master-detail over two
sections is a single page split for no benefit. A profile may name section roots explicitly at
v0.2, but never as the default, because the default must be a total function.

### 5.2 Traversal

Free navigation by default: any section or step is reachable at any time.

A **Traversal** may skip a step whose nodes do not exist. **It may never withhold forward movement
because a value is missing or violates the Schema** (ADR 0004 decision 5). Where guidance is
warranted, a step or section shows the count of **Absent** `required` **Fields** and of
**Violations** beneath it, and the forward action stays primary. The counts are a projection of the
existing **Violation** set grouped by **Partition** — the same data as design §11's C6 summary, not
a new computation.

### 5.3 Form search

**Form search** is fuzzy matching over **Form node** titles, descriptions, and **Paths**, and it
lives in `confyg-form` as a pure function (`fuzzy_matcher::skim`, the crate upstream already uses
in `confy-core/src/session/search.rs:10-34`). It is not taken from `session::`, which ADR 0001
gates off.

It lives in the compiler rather than the hosts for two reasons: a result must be able to move the
**Partition** to the section containing the hit, which is a cross-layer interaction; and a
host-side implementation guarantees the Web and TUI search semantics drift apart.

## 6. Lexicon

Two kinds of string, deliberately separated:

| Kind | Content | Carrier | Lifecycle |
|---|---|---|---|
| Chassis | buttons, dialogs, notices, widget names | flat catalog, `i18n/{en,zh-TW}.json`, prefix `form.*` | confyg's version |
| Schema content | `title`, `description` of a specific Schema | `lexicon/<schema-id>.<lang>.json`, keyed by Schema pointer | the Schema's version |

They are not merged into one catalog. Their lifecycles differ, so mixing them breaks the key-parity
guarantee upstream enforces by test (`confy-core/src/session/i18n.rs:136-150`); Schema pointers
contain `/` and `~1` escapes that collide with a dot-delimited key space; and a Schema's
translation should be distributable on its own, so that a Chinese pack for one published Schema is
a file someone can ship without touching confyg.

The profile's `lexicon` section is a third thing again: in-place override of one node's label or
help (`x-confyg.label` / `.help` by another carrier), not translation.

Chassis conventions follow upstream exactly — flat dot-delimited keys, `{0}` positional
substitution, fallback active → `en` → the raw key, never panicking
(`confy/web/i18n.ts:60-89`, `confy-core/src/session/i18n.rs:69-83`).

Degradation is explained by **one** template plus a widget-name key set:

- `form.degrade.generic` — "This environment cannot render {0}; showing {1} instead."
- `form.widget.<name>` — one per **Widget**, needed anyway for the affordance override UI.

A per-widget degradation message was rejected: **Partition** and **Widget** values are closed
before they are implemented (§5.1, §3.1), so per-value copy would demand translations for `wizard`
two releases before it renders.

## 7. Appearance

**Appearance tokens** extend upstream's semantic OKLCH set (`confy/web/style.css:6-69`) rather
than replacing it: the 20 chrome roles, `--font` / `--mono`, and the density tokens `--row-h`,
`--indent`, `--hit` are inherited as-is, and confyg's themes are the same two `data-theme`
attributes.

Not inherited: the data-type colours `--t-string`, `--t-number`, `--t-bool`, `--t-date`,
`--t-null`, `--t-branch`. They serve a tree editor where type is otherwise invisible. In a form the
label and the **Widget** carry the type, and colouring by type would be noise.

Added, because confyg has states upstream does not:

| Token | Role |
|---|---|
| `--ghost` | **Ghost text**: an **Absent** **Field**'s inherited default |
| `--inherited` | the "inherit" option inside a control (ADR 0003) |
| `--violation` | a **Violation** — the validator actually reported it |
| `--notice` | a **Notice** — informational, not a rule failure |
| `--locked` | a **Locked** node's non-writable value |
| `--required` | the `required` marker |
| `--deprecated` | a `deprecated` **Field**'s badge |
| `--radius` | corner radius, which upstream hardcodes per rule rather than tokenizing |

Splitting upstream's single `--warn` into `--violation` and `--notice` happens **in confyg only**;
`--warn` stays as it is in confy, and no change is sent upstream. The distinction is load-bearing
here — the glossary keeps **Violation** and **Notice** apart precisely so a preserved
**Unknown key** never looks like a rule failure — and it is invisible in a tree editor.

`background_media` is host-optional and degrades to nothing. A decorative image is the clearest
case of an **Appearance** decision that cannot live in a **Schema**: it has no relationship to any
subschema (ADR 0005 decision 4).

## 8. Verification

Design §11 gains a sixth item, and this record owns it:

6. **Write-neutrality.** For a fixed `(Schema, Document, [Setter intent])`, running the whole
   pipeline under *N* different presentation profiles × *M* different `HostProfile` values must
   produce **byte-identical** output. This is a property test over presentation inputs, not
   another fixture matrix: it asserts that output is invariant under everything in layers 3–6, and
   it pins `HostProfile` as pure data as a side effect.

Two existing items widen:

- *Compiler snapshots* (item 1) gain a `HostProfile` axis, so a clamped **Widget** and its
  retained `intended` are asserted rather than inspected.
- *Presence and Occupancy matrix* (item 3) covers each **Widget**'s terminal fallback, since a
  degraded control must still express all three **Presence** states.

## 9. Tiers

| Tier | Presentation work |
|---|---|
| v0.1 | the eight layers; **Write-neutrality** and its test; the nine-member vocabulary as `x-confyg`; `HostProfile` and compile-time clamp; `scroll` and `sections`; **Form search**; the added **Appearance tokens**; `filterable-menu` clamped to `menu` |
| v0.2 | the **Presentation profile** sidecar with **Profile hint** and sibling discovery; `tabs`; the combobox implementing `filterable-menu`; **Lexicon** Schema translation tables; explicit section roots |
| v0.3 | `wizard`; the TUI `HostProfile` |

v0.1 therefore has **no sidecar**, and **Flow** and **Appearance** have no override entry point in
v0.1. This is deliberate: the shape a profile should take is best judged against two or more real
deployments, and the cost of deferring is low because the vocabulary is closed and the resolution
chain is fixed now (ADR 0005 decision 10).

## 10. Open questions

- The `Density` values' concrete mapping to upstream's `--row-h` / `--indent` / `--hit` scales is
  not fixed. Upstream has three de facto densities (desktop 30px, phone 38px, touch 54px) reached
  through media and container queries, not through a token; confyg needs `Density` to select one
  explicitly for the TUI's benefit. Must be settled before the v0.3 TUI host.
- *Preset value sets* (design §7 C10) have no carrier. They are not a profile section, because
  they write bytes — only ever through explicit **Setter intents**, but admitting them would make
  the profile's write-neutral reading conditional. To be designed with v0.3.
