# 0004 — The presentation layer model and Write-neutrality

Date: 2026-09-04

[ADR 0002](0002-schema-driven-projection-and-three-state-presence.md) established that the
**Schema** drives projection, and [ADR 0003](0003-form-ir-shape-presence-scope-and-default-equivalence.md)
settled the shape of the **Form IR**. Both answered *what a form contains*. Neither answered *what
decides how it looks, how it is laid out, and who is allowed to decide* — and the design record
had accumulated four unrelated answers to that question: a `Widget` precedence list (§3), a
parenthetical about option counts (§7 A5), an inherited token set (§10), and one open question
about the **Annotation** vocabulary (§12). This ADR names the layers, fixes their authority, and
states the invariant that keeps them separable.

Which *files* may carry a presentation decision is a separate decision, recorded in
[ADR 0005](0005-presentation-profile-as-a-second-carrier.md).

## Decision

**1. Eight layers, two of them closed.**

Every presentation decision confyg makes belongs to exactly one of these layers:

| Layer | Governs | Authority |
|---|---|---|
| **Value contract** | JSON Schema keywords: type, bounds, `enum`, `pattern`, `required`, `default` | The Schema author |
| Structure (the **Form IR**) | **Form node** kind, nesting, order | Derived; never hand-written |
| **Affordance** | Which **Widget** a **Field** resolves to | Derived; overridable |
| **Flow** | How the **Form IR** is partitioned into screens and how the user moves between them | Overridable |
| **Lexicon** | Labels, descriptions, help text, units, translations | Overridable |
| **Appearance** | Semantic tokens, theme, density | Overridable |
| **Emission style** | The concrete syntax written for new text | **Closed** |
| **Conduct** | Keyboard, focus, selection, scrolling | **Closed** |

A **closed** layer has no author and no override entry point — not an **Annotation**, not a
sidecar, not a user preference. Adding one requires an ADR superseding this one.

**2. Write-neutrality.** No layer above **Value contract** and **Emission style** may change the
bytes written to the **Document**. Presentation decides what the user sees and how they express an
intent; it never decides what that intent writes. This is an invariant with a test, not a
guideline — see Consequences.

**3. Derivation thresholds are not configurable; derivation results are overridable.** The numeric
cut-off that turns a 13-option `enum` into a filterable menu is an implementation detail of the
**Affordance** derivation, not a design intent, and is therefore a constant. The *outcome* is
addressable per node. A deployment that wants a typeable 8-option menu says so about that node; it
cannot retune the global rule and change every other node as a side effect.

**4. Flow is three concepts, and only two of them are Flow.** *Partition* (how the tree is cut
into screens) and *Traversal* (how the user moves between them) are **Flow**. Data-driven
structure — `if` / `then` / `else`, `dependentSchemas`, which is why setting one value can make
another **Field** exist (§4 step 3) — is **Value contract**. They are never merged: the first two
choose among nodes that exist, the third decides which nodes exist at all.

**5. Flow never gates on validity.** A **Traversal** may skip a step whose nodes do not exist. It
may never withhold forward movement because a value is missing or violates the Schema. Where
guidance is warranted, the step displays the count of unfilled and violating nodes and leaves the
forward action primary.

**6. Host capability is pure data, and clamping happens at compile time.** A host declares what it
can render as a `HostProfile` value — a flat set of capability facts and a density, handed over
once at session start. `confyg-form` compiles `(Schema, Document, profile, HostProfile) → FormNode`
and emits the **Widget** the host can actually render, while retaining the **Widget** the
derivation intended, so the form can explain the substitution honestly. Each **Widget** declares a
**Degradation ladder** — an ordered fallback chain terminating in a universally available control.

## Why

**Why layers at all.** The four scattered answers were not merely untidy. They had no shared rule,
so each new question ("can a schema pick the theme?", "can a deployment hide a field?", "does the
TUI get a slider?") had to be answered from taste. A taxonomy converts those into lookups: the
question is which layer the knob belongs to, and each layer's authority is already decided.

**Why Emission style is closed.** It is the one presentation-adjacent layer that changes bytes.
Opening it would make **Write-neutrality** unstateable, because the invariant's whole content is
"exactly two layers touch the file". Its policy is already fixed for a reason recorded in §1: the
notation freedom confyg drops relative to confy *is* the product decision.

**Why Conduct is closed.** Keyboard and focus behaviour is a property of the host platform, not of
the configuration being edited. A Schema author who could specify focus order would be overriding
`wens-dev-principles ui` from inside a data file, with no way for the host to reconcile the two.

**Why Write-neutrality is the load-bearing invariant.** Without it, "presentation" has no
boundary, and every layer becomes a place where a config file's meaning can be altered at a
distance. With it, the entire presentation stack is untrusted input: a broken, hostile, or simply
outdated presentation decision can make a form ugly, awkward, or badly organized, and cannot make
it write the wrong file. It also makes the layer split *testable* rather than architectural
folklore — the invariant is a property over arbitrary presentation inputs.

**Why thresholds are constants.** The failure mode of a tunable threshold is silent divergence:
the same Schema renders differently in two deployments, and nothing in either file explains why,
because the setting that caused it does not mention any of the affected nodes. A per-node override
names its subject.

**Why Flow must not gate.** **Soft constraint** is stated in terms of input and saving, and a
wizard gate technically violates neither — it blocks *navigation*. That is the reasoning that
smuggles a validation gate into a design that forbids them. The gate's practical effect is worse
than a rejected save: the user cannot even reach the field that would fix the problem. Displaying
counts delivers the guidance a wizard exists to provide, and reuses the **Violation** summary
(C6) rather than adding a mechanism.

**Why clamp at compile time.** The alternative distributes the **Degradation ladder** walk to
every host, which guarantees the Web and TUI hosts diverge — the same argument that puts
**Form search** in `confyg-form`. Clamping in the compiler also puts host degradation inside the
snapshot test surface (§11), so "what does this form look like on the TUI" is an assertion rather
than a manual check.

## Alternatives rejected

**Two layers: "schema" and "UI".** The split everyone reaches for first. Rejected because it puts
**Flow**, **Appearance**, and **Emission style** in one bucket, and **Emission style** is the only
one that writes bytes — the distinction the entire design depends on would be the one the taxonomy
erases.

**Widening Soft constraint to permit navigation gates.** Would legitimize wizard gating by
narrowing the principle to input and save. Rejected: the principle's purpose is that confyg never
stands between the user and their file, and a gate is the most obtrusive possible form of standing
between them.

**A configurable option-count threshold.** Rejected for the silent-divergence failure above.

**Capability as trait methods on the host.** Mirrors `confy-core`'s `Host` trait, which is
precedent of a kind. Rejected because capability is a static fact, not a behaviour: as methods it
would be re-interrogated per widget, could answer inconsistently within one render, and could not
be serialized into a compiler snapshot.

**Rendering the ideal Widget and letting hosts fall back.** Keeps the compiler simpler and the IR
more "pure". Rejected for the host-divergence argument above; the IR stays host-independent
because `HostProfile` is an explicit input, not because the IR pretends hosts are identical.

**Letting the presentation layer hide a node outright.** Deferred to
[ADR 0005](0005-presentation-profile-as-a-second-carrier.md), which rejects it: a hidden
`required` **Field** with no `default` makes a file that cannot be made valid from the UI, with no
visible cause.

## Consequences

- §11 gains a sixth verification: the same `(Schema, Document, [Setter intent])` run under
  multiple presentation profiles and multiple `HostProfile` values must produce byte-identical
  output. This is **Write-neutrality**'s test, and it pins `HostProfile` as pure data as a
  side effect.
- The **Form IR** carries both the clamped and the intended **Widget** on every **Field**. Hosts
  render the former and may explain the latter.
- Every **Widget** in §7 must declare a **Degradation ladder**. A **Widget** with no fallback
  chain to a universally available control is not admissible.
- `confyg-form` grows two input parameters and stays a pure function with no I/O, which keeps the
  compiler as the single unit-test surface.
- Guidance counts on **Flow** steps are a projection of the existing **Violation** set, not a new
  computation.
- confy's data-type colours (`--t-string`, `--t-number`, and the rest) are not inherited. They
  serve a tree editor where type is otherwise invisible; in a form, the label and the **Widget**
  carry it, and colouring by type would be noise. confyg extends confy's chrome tokens with the
  roles its own states need — inherited **Ghost text**, **Locked**, **Violation** as distinct from
  **Notice**, required, deprecated.
