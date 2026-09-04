# confyg v0.1 Implementation Plan
Status: Draft

> **For agentic workers:** REQUIRED SUB-SKILL: use `subagent-driven-development` (recommended) or
> `executing-plans` to implement this plan task by task. Steps use checkbox (`- [ ]`) syntax for
> tracking. Every task ends with a commit; per `CLAUDE.md`, that commit carries the code, the
> `CHANGELOG.md` entry, and any documentation update together.

**Goal:** ship confyg v0.1 — given a JSON Schema and optionally a config file, render a form that
makes the file correct to fill in, and write only what the user decided, byte-preserving
everything else.

**Architecture:** a pure `Schema → Form IR` compiler (`confyg-form`, no I/O, no state), a session
layer that overlays the **Document**, dispatches **Setter intents**, and lowers them onto
`confy-core` `Mutation`s (`confyg-session`), a WASM boundary that exposes exactly one call
(`confyg-ffi`), and a TypeScript renderer that walks the IR (`web/`). The compiler is the unit-test
surface; the lowering is the round-trip-test surface.

**Tech Stack:** Rust 2021 workspace, `confy-core` (pinned git dep, `default-features = false`),
`jsonschema` 0.30, `serde_json` with `preserve_order`, `insta` for snapshots, `wasm-bindgen`,
TypeScript + Vite for `web/`.

**Specs:** [`../spec/2026-09-03-confyg-design.md`](../spec/2026-09-03-confyg-design.md) (Form IR,
intents, patterns, write policy) and
[`../spec/2026-09-04-presentation-layers-design.md`](../spec/2026-09-04-presentation-layers-design.md)
(layers, vocabulary, ladders, partition). Cited below as *design §n* and *presentation §n*.
Volatile upstream facts: [`../reference/upstream.md`](../reference/upstream.md). Vocabulary:
[`../reference/glossary.md`](../reference/glossary.md) — every bolded term has an entry there.

## Global Constraints

- `confy-core = { git = "https://github.com/superyngo/confy", rev = "<sha>", default-features = false }`.
  `Cargo.lock` is committed. No release tag supports confyg yet; `tag = "v1.1.0"` replaces the rev
  at v0.1 release (`upstream.md` *The pin*). `tag = "v1.0.1"` does not exist.
- `serde_json = { version = "1", features = ["preserve_order"] }` is **mandatory**: without it a
  Schema's `properties` arrive alphabetically and every form is scrambled (design §2, D6).
- `confy-core`'s `session::` module is never referenced. Zero occurrences of `confy_core::session`
  in confyg; a grep for it is part of Task 1's acceptance.
- `confyg-form` performs no I/O and holds no state: pure functions over `(&serde_json::Value, &str)`.
- `confyg-session` performs no I/O either. It emits a `SchemaFetchRequest`; the host answers with
  the `LoadSchema` **Session command** (design §6).
- Live `pattern` checks use the validator's own engine (`fancy-regex` via `jsonschema`), never Rust
  `regex`: *form warnings and Violations never disagree* (design §7, `upstream.md`).
- Every mutation passes `OnCollision::Cancel` and an explicit `suggested_key`. A generic
  `placeholder` key may never appear in a confyg-written file (design §5, `upstream.md`
  *Fragment contract*).
- **Soft constraint**: a value violating the Schema is written and warned about, never refused. No
  step in this plan may add a write gate.
- **Write-neutrality**: nothing in the **Affordance**, **Flow**, **Lexicon**, or **Appearance**
  layer may change written bytes (presentation §1, ADR 0004). Task 17 tests it.
- **Minimal write**: a value equal to the effective Schema `default` is not written; `SetValue`
  with such a value lowers to `Delete` (design §8, ADR 0003).
- Every `Insert` converts a **projection** index to a comments-included child ordinal first. This
  is D7, whose failure mode is misplaced text rather than an error (`upstream.md` *Index spaces*).
- Out of v0.1 scope, and no task may add them: **Map group**, **Tuple group**, **Variant group**,
  **Union field**, `MoveRepeatItem`, `Remark`, diff preview, catalog matching, Draft 7
  normalization, external `$ref`, the TUI host, `tabs`, `wizard`, `filterable-menu` as a real
  control (it clamps to `menu` in v0.1, presentation §3.1).

## File structure

| Path | Responsibility |
|---|---|
| `Cargo.toml`, `rust-toolchain.toml` | workspace, pinned dependency, shared lints |
| `crates/confyg-form/src/ir.rs` | `FormNode`, `Presence`, `Occupancy`, `NodeMeta`, `FieldMeta`, `Widget`, serde |
| `crates/confyg-form/src/facts.rs` | Schema keyword introspection → `SchemaFacts` |
| `crates/confyg-form/src/vocab.rs` | the nine-member **Presentation vocabulary**, `x-confyg` parsing |
| `crates/confyg-form/src/affordance.rs` | **Widget** derivation, menu family, **Degradation ladder**, `HostProfile` clamp |
| `crates/confyg-form/src/compile.rs` | design §4 steps 1–5: resolve, classify, order |
| `crates/confyg-form/src/overlay.rs` | step 6: **Presence** / **Occupancy**, `locked`, **Violation** attribution |
| `crates/confyg-form/src/unknown.rs` | step 7 sweep and the `additionalProperties` decision table |
| `crates/confyg-form/src/constraint.rs` | `Constraint` extraction and guidance text |
| `crates/confyg-session/src/ordinal.rs` | projection index → child ordinal; root-slot computation |
| `crates/confyg-session/src/lower.rs` | **Setter intent** → `Mutation`, including **Absent-parent lowering** |
| `crates/confyg-session/src/template.rs` | **Template** generation, **Comment policy**, **Schema hint** emission |
| `crates/confyg-session/src/session.rs` | session state, dispatch, `SetterSnapshot`, undo/redo ring |
| `crates/confyg-ffi/src/lib.rs` | one `dispatch` export, JSON in / JSON out |
| `web/src/render.ts` | IR walk → DOM; one module per **Widget** family under `web/src/widgets/` |
| `web/src/host-io.ts` | open / save / format dialog, ported from confy |
| `tests/` per crate | snapshots (`insta`), the round-trip matrix, the property tests |

---

## Phase A — the Rust core

### Task 1: Workspace, pin, and the "upstream really works" test

**Files:**
- Create: `Cargo.toml`, `rust-toolchain.toml`, `crates/confyg-form/Cargo.toml`,
  `crates/confyg-form/src/lib.rs`, `crates/confyg-form/tests/upstream_pin.rs`
- Create: `.github/workflows/ci.yml`

**Interfaces:**
- Produces: the workspace; every later task adds crates as members.

- [ ] **Step 1: Resolve the pin.** `git ls-remote https://github.com/superyngo/confy main` and use
      that sha as `<sha>`. Record it in `upstream.md` *The pin* in this task's commit.

- [ ] **Step 2: Write the failing test** — `crates/confyg-form/tests/upstream_pin.rs`:

```rust
use confy_core::model::{AnyDocument, ConfigDocument, DocFormat};

#[test]
fn upstream_parses_and_serializes_byte_identically() {
    let src = "# lead\n[server]\nhost = \"a\"  # trail\n";
    let doc = AnyDocument::from_str_as(src, DocFormat::Toml).expect("parse");
    assert_eq!(doc.serialize(), src, "untouched text must round-trip byte-identically");
}

#[test]
fn confy_session_is_not_linked() {
    // The session feature is gated off (ADR 0001). This is asserted by CI's grep step;
    // the compile-time half is that this crate builds with default-features = false.
}
```

- [ ] **Step 3: Run it and watch it fail.** `cargo test -p confyg-form` → FAIL, no such crate /
      unresolved import.

- [ ] **Step 4: Write the workspace.**

```toml
# Cargo.toml
[workspace]
members = ["crates/confyg-form"]
resolver = "2"

[workspace.dependencies]
confy-core = { git = "https://github.com/superyngo/confy", rev = "<sha>", default-features = false }
serde = { version = "1", features = ["derive"] }
serde_json = { version = "1", features = ["preserve_order"] }
jsonschema = { version = "0.30", default-features = false }
insta = { version = "1", features = ["json"] }
```

- [ ] **Step 5: Run it and watch it pass.** `cargo test -p confyg-form`. If `serialize` is not
      callable, `ConfigDocument` is missing from the `use` — that trait must be in scope
      (`upstream.md` *Reachable API*).

- [ ] **Step 6: Add the CI grep gate** to `.github/workflows/ci.yml`, after `cargo test`:

```yaml
      - name: confy-core session must not be referenced
        run: |
          if rg -q 'confy_core::session' crates/; then echo "session module referenced"; exit 1; fi
```

- [ ] **Step 7: Commit.**

```bash
git add -A && git commit -m "feat: scaffold the workspace on a pinned confy-core

Pins confy-core by revision with default-features = false per ADR 0001, enables
serde_json preserve_order globally per design §2, and asserts both halves of the
pin: a byte-identical TOML round-trip through the public API, and a CI gate that
fails if confy_core::session is ever referenced."
```

---

### Task 2: The Form IR types

**Files:**
- Create: `crates/confyg-form/src/ir.rs`, `crates/confyg-form/tests/ir_serde.rs`
- Modify: `crates/confyg-form/src/lib.rs`

**Interfaces:**
- Produces: `FormNode`, `Presence`, `Occupancy`, `NodeMeta`, `FieldMeta`, `Widget`, `Bounds`,
  `GroupToggle`, `TemplateRef`, `Constraint`, `SchemaState`. Every later task consumes these.
  Serialization is stable and snapshot-friendly: `#[serde(tag = "kind", rename_all = "camelCase")]`
  on `FormNode`, because `web/` and `insta` both read it.

- [ ] **Step 1: Write the failing test** — `tests/ir_serde.rs`:

```rust
use confyg_form::ir::*;

#[test]
fn absent_field_serializes_with_its_default_and_intended_widget() {
    let node = FormNode::Field {
        path: vec![],
        widget: Widget::Menu,
        intended: Widget::FilterableMenu,
        presence: Presence::Absent { default: Some(serde_json::json!("info")), remarked: None },
        meta: FieldMeta::default(),
    };
    let v = serde_json::to_value(&node).unwrap();
    assert_eq!(v["kind"], "field");
    assert_eq!(v["widget"], "menu");
    assert_eq!(v["intended"], "filterableMenu");
    assert_eq!(v["presence"]["kind"], "absent");
    assert_eq!(v["presence"]["default"], "info");
}

#[test]
fn occupancy_distinguishes_empty_from_absent() {
    assert_ne!(
        serde_json::to_value(Occupancy::Empty).unwrap(),
        serde_json::to_value(Occupancy::Absent).unwrap(),
        "servers = [] and a missing servers key are different facts (ADR 0003)"
    );
}
```

- [ ] **Step 2: Run it, watch it fail.** `cargo test -p confyg-form --test ir_serde`.

- [ ] **Step 3: Write `ir.rs`,** transcribing design §3's enum verbatim and adding the
      presentation-layer fields. `Widget` is a closed enum; its variants are exactly the names in
      presentation §4's ladder table.

```rust
use confy_core::model::Path;
use confy_core::schema::Violation;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum FormNode {
    Field { path: Path, widget: Widget, intended: Widget, presence: Presence, meta: FieldMeta },
    Group { path: Path, meta: NodeMeta, children: Vec<FormNode>, occupancy: Occupancy,
            toggle: Option<GroupToggle> },
    Repeat { path: Path, meta: NodeMeta, items: Vec<FormNode>, occupancy: Occupancy,
             bounds: Bounds, item_template: TemplateRef, label_from: Option<String> },
    Unknown { path: Path, raw_preview: String },
    Cyclic { path: Path, schema_ptr: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Presence {
    Absent { default: Option<Value>, remarked: Option<String> },
    Set { literal: String },
    Invalid { literal: String, violations: Vec<Violation> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Occupancy { Absent, Empty, Populated }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Widget {
    Text, RawText, DisplayOnly, Radio, Menu, FilterableMenu, CheckboxSet,
    Tristate, Stepper, Slider, Textarea, Masked,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeMeta {
    pub title: String,
    pub description: Option<String>,
    pub violations: Vec<Violation>,
    pub locked: Option<Locked>,
    pub deprecated: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldMeta {
    #[serde(flatten)]
    pub node: NodeMeta,
    pub default: Option<Value>,
    pub examples: Vec<Value>,
    pub required: bool,
    pub read_only: bool,
    pub write_only: bool,
    pub unit: Option<String>,
    pub constraints: Vec<Constraint>,
    pub raw: bool,
}
```

      `Bounds { min: Option<usize>, max: Option<usize> }`; `TemplateRef(pub String)` is a JSON
      pointer into the Schema, not inlined text, so the IR stays small;
      `GroupToggle { enabled: bool }`; `Locked { reason: LockedReason }` with
      `LockedReason::{YamlAlias, MergeKey}`; `SchemaState { projected: bool, validatable: Result<(), SchemaCompileError> }`.

- [ ] **Step 4: Run it, watch it pass.**

- [ ] **Step 5: Commit.** `feat(form): define the Form IR types`.

---

### Task 3: Schema keyword introspection

**Files:**
- Create: `crates/confyg-form/src/facts.rs`, `crates/confyg-form/tests/facts.rs`

**Interfaces:**
- Consumes: nothing.
- Produces: `pub fn facts(schema: &Value) -> SchemaFacts` where
  `SchemaFacts { ty: Option<TypeSet>, default: Option<Value>, examples: Vec<Value>, enum_values: Option<Vec<Value>>, const_value: Option<Value>, title: Option<String>, description: Option<String>, deprecated: bool, read_only: bool, write_only: bool, format: Option<String>, bounds: NumBounds, len: LenBounds, pattern: Option<String>, multiple_of: Option<f64>, unique_items: bool, additional: AdditionalProperties, required: Vec<String>, prefix_items: Option<Vec<Value>> }`.
  `AdditionalProperties::{Schema(Value), Open, Closed}` encodes design §7's three-form table.

  This is the eight-keyword set `schema::hints_edit` does not read, which is why it is confyg's
  (`upstream.md` *The upstream bill*).

- [ ] **Step 1: Write the failing test** — `tests/facts.rs`:

```rust
use confyg_form::facts::{facts, AdditionalProperties};

#[test]
fn reads_the_eight_keywords_hints_edit_ignores() {
    let s = serde_json::json!({
        "type": "integer", "default": 8080, "examples": [80, 443],
        "readOnly": true, "deprecated": true, "minimum": 1, "maximum": 65535,
        "multipleOf": 1
    });
    let f = facts(&s);
    assert_eq!(f.default, Some(serde_json::json!(8080)));
    assert_eq!(f.examples.len(), 2);
    assert!(f.read_only && f.deprecated);
    assert_eq!((f.bounds.min, f.bounds.max), (Some(1.0), Some(65535.0)));
}

#[test]
fn additional_properties_has_three_forms() {
    let obj = serde_json::json!({"additionalProperties": {"type": "string"}});
    assert!(matches!(facts(&obj).additional, AdditionalProperties::Schema(_)));
    assert!(matches!(facts(&serde_json::json!({})).additional, AdditionalProperties::Open));
    assert!(matches!(
        facts(&serde_json::json!({"additionalProperties": false})).additional,
        AdditionalProperties::Closed
    ));
}

#[test]
fn property_order_follows_the_schema_file() {
    let s: serde_json::Value = serde_json::from_str(
        r#"{"properties":{"host":{},"port":{},"ca_cert":{}}}"#).unwrap();
    let keys: Vec<_> = s["properties"].as_object().unwrap().keys().cloned().collect();
    assert_eq!(keys, ["host", "port", "ca_cert"], "preserve_order is not enabled (D6)");
}
```

- [ ] **Step 2: Run it, watch it fail.**
- [ ] **Step 3: Implement `facts.rs`** as one pass over the object, defaulting every field. Never
      panic on a malformed Schema: an unexpected type reads as absent, because design §4 must
      produce a complete form from any input.
- [ ] **Step 4: Run it, watch it pass.**
- [ ] **Step 5: Commit.** `feat(form): introspect the Schema keywords confy-core skips`.

---

### Task 4: The Presentation vocabulary

**Files:**
- Create: `crates/confyg-form/src/vocab.rs`, `crates/confyg-form/tests/vocab.rs`

**Interfaces:**
- Produces: `pub struct Presentation { affordance: Option<Widget>, order: Option<i64>, unit: Option<String>, collapsed: Option<bool>, demoted: Option<bool>, label: Option<String>, help: Option<String>, label_from: Option<String>, option_labels: Option<BTreeMap<String, String>> }`,
  `pub fn read(schema: &Value) -> (Presentation, Vec<Notice>)`, and
  `pub fn profile_hint(root: &Value) -> Option<String>`.
  All nine members are optional; the struct's `Default` is the empty override. Presentation §2 is
  the authoritative table and this struct must match it member for member.

- [ ] **Step 1: Write the failing test** — `tests/vocab.rs`:

```rust
use confyg_form::{ir::Widget, vocab};

#[test]
fn reads_all_nine_members() {
    let s = serde_json::json!({"x-confyg": {
        "affordance": "stepper", "order": 3, "unit": "MiB", "collapsed": true,
        "demoted": true, "label": "Cache size", "help": "per worker",
        "labelFrom": "name", "optionLabels": {"aes-256-gcm": "AES-256-GCM"}
    }});
    let (p, notices) = vocab::read(&s);
    assert_eq!(p.affordance, Some(Widget::Stepper));
    assert_eq!(p.order, Some(3));
    assert_eq!(p.unit.as_deref(), Some("MiB"));
    assert!(notices.is_empty());
}

#[test]
fn an_unknown_member_is_a_notice_not_an_error() {
    let s = serde_json::json!({"x-confyg": {"hidden": true}});
    let (p, notices) = vocab::read(&s);
    assert!(p.affordance.is_none());
    assert_eq!(notices.len(), 1, "there is no `hidden`; unknown keys are Notices (ADR 0005)");
}

#[test]
fn profile_hint_lives_at_the_root_only() {
    let root = serde_json::json!({"x-confyg": {"profile": "./app.confyg.toml"}});
    assert_eq!(vocab::profile_hint(&root).as_deref(), Some("./app.confyg.toml"));
    let (p, notices) = vocab::read(&root);
    assert!(p.affordance.is_none() && notices.is_empty(),
        "profile is not a vocabulary member and must not warn");
}
```

- [ ] **Step 2: Run it, watch it fail.**
- [ ] **Step 3: Implement.** Deserialize with `#[serde(deny_unknown_fields)]` on a shadow struct
      and, on failure, fall back to a permissive pass that collects unknown keys as `Notice`s —
      an unknown key must never cost the known ones (**Soft constraint** extended to the profile,
      ADR 0005).
- [ ] **Step 4: Run it, watch it pass.**
- [ ] **Step 5: Commit.** `feat(form): parse the nine-member Presentation vocabulary`.

---

### Task 5: Affordance derivation, the menu family, and the HostProfile clamp

**Files:**
- Create: `crates/confyg-form/src/affordance.rs`, `crates/confyg-form/tests/affordance.rs`

**Interfaces:**
- Consumes: `facts::SchemaFacts`, `vocab::Presentation`, `ir::Widget`.
- Produces:
  `pub struct HostProfile { pub can_mask: bool, pub can_slide: bool, pub can_filter_options: bool, pub density: Density }`,
  `pub fn derive(f: &SchemaFacts, raw: bool) -> Widget`,
  `pub fn ladder(w: Widget) -> &'static [Widget]`,
  `pub fn resolve(f: &SchemaFacts, p: &Presentation, raw: bool, host: &HostProfile) -> (Widget, Widget, Vec<Notice>)`
  returning `(widget, intended, notices)`.

- [ ] **Step 1: Write the failing test** — `tests/affordance.rs`:

```rust
use confyg_form::{affordance::*, facts::facts, ir::Widget, vocab};

fn host() -> HostProfile {
    HostProfile { can_mask: true, can_slide: true, can_filter_options: false,
                  density: Density::Desktop }
}

#[test]
fn menu_family_thresholds_are_constants() {
    let mk = |n: usize| {
        let vals: Vec<_> = (0..n).map(|i| serde_json::json!(i)).collect();
        facts(&serde_json::json!({"enum": vals}))
    };
    assert_eq!(derive(&mk(4), false), Widget::Radio);
    assert_eq!(derive(&mk(5), false), Widget::Menu);
    assert_eq!(derive(&mk(12), false), Widget::Menu);
    assert_eq!(derive(&mk(13), false), Widget::FilterableMenu);
}

#[test]
fn precedence_puts_raw_fallback_above_the_override() {
    let f = facts(&serde_json::json!({"type": "integer", "minimum": 0, "maximum": 9}));
    let p = vocab::read(&serde_json::json!({"x-confyg": {"affordance": "text"}})).0;
    assert_eq!(resolve(&f, &p, true, &host()).0, Widget::RawText);
    assert_eq!(resolve(&f, &p, false, &host()).0, Widget::Text, "override beats derivation");
    assert_eq!(resolve(&f, &Default::default(), false, &host()).0, Widget::Slider,
        "both bounds present derives a slider");
}

#[test]
fn clamp_substitutes_but_keeps_intended() {
    let f = facts(&serde_json::json!({"type": "string", "writeOnly": true}));
    let h = HostProfile { can_mask: false, ..host() };
    let (w, intended, notices) = resolve(&f, &Default::default(), false, &h);
    assert_eq!((w, intended), (Widget::Text, Widget::Masked));
    assert_eq!(notices.len(), 1,
        "a writeOnly value shown unmasked must say so, never silently (design §7 A4)");
}

#[test]
fn filterable_menu_clamps_to_menu_in_v0_1() {
    let vals: Vec<_> = (0..400).map(|i| serde_json::json!(i)).collect();
    let f = facts(&serde_json::json!({"enum": vals}));
    let (w, intended, _) = resolve(&f, &Default::default(), false, &host());
    assert_eq!((w, intended), (Widget::Menu, Widget::FilterableMenu));
}
```

- [ ] **Step 2: Run it, watch it fail.**
- [ ] **Step 3: Implement** `derive` as presentation §3's six-step precedence, `ladder` as
      presentation §4's table (`slider → stepper → text`, `masked → text` with a mandatory
      **Notice**, `filterable-menu → menu → radio → text`, `swatch → text`, and every ladder
      terminating in `text` or `display-only`), and `resolve` as
      derive → override → clamp. The clamp walks the ladder until the host can render a rung; it
      never re-resolves.
- [ ] **Step 4: Run it, watch it pass.**
- [ ] **Step 5: Commit.** `feat(form): resolve Widgets and clamp them to host capability`.

---

### Task 6: Compilation steps 1–5 — resolve, classify, order

**Files:**
- Create: `crates/confyg-form/src/compile.rs`, `crates/confyg-form/src/constraint.rs`,
  `crates/confyg-form/tests/compile.rs`, `crates/confyg-form/tests/fixtures/*.json`
- Modify: `crates/confyg-form/src/lib.rs`

**Interfaces:**
- Produces: `pub fn compile(schema: &Value, host: &HostProfile) -> Compiled` where
  `Compiled { root: FormNode, state: SchemaState, notices: Vec<Notice> }`. Document overlay is
  Task 7; this task produces the all-**Absent** tree.

- [ ] **Step 1: Write the failing tests** — `tests/compile.rs`:

```rust
#[test]
fn properties_become_a_group_in_schema_order_with_no_required_hoisting() {
    let s: serde_json::Value = serde_json::from_str(
        r#"{"type":"object","required":["port"],
            "properties":{"host":{"type":"string"},"port":{"type":"integer"}}}"#).unwrap();
    let c = compile(&s, &host());
    let keys = child_keys(&c.root);
    assert_eq!(keys, ["host", "port"], "the Schema author's order wins (design §4 step 5)");
}

#[test]
fn x_confyg_order_overrides_and_demoted_sinks() {
    let s: serde_json::Value = serde_json::from_str(
        r#"{"properties":{"a":{"x-confyg":{"order":2}},"b":{"x-confyg":{"order":1}},
                          "c":{"x-confyg":{"demoted":true}}}}"#).unwrap();
    assert_eq!(child_keys(&compile(&s, &host()).root), ["b", "a", "c"]);
}

#[test]
fn a_scalar_array_is_a_repeat_of_fields_not_a_field() {
    let s = serde_json::json!({"type":"array","items":{"type":"string"},
                               "minItems":1,"maxItems":3});
    match compile(&s, &host()).root {
        FormNode::Repeat { bounds, item_template, .. } => {
            assert_eq!((bounds.min, bounds.max), (Some(1), Some(3)));
            assert_eq!(item_template.0, "#/items");
        }
        other => panic!("ADR 0003: a scalar array is a Repeat group, got {other:?}"),
    }
}

#[test]
fn ref_and_all_of_are_resolved_and_a_cycle_terminates() {
    let s: serde_json::Value = serde_json::from_str(
        r#"{"$defs":{"n":{"type":"object","properties":{"child":{"$ref":"#/$defs/n"}}}},
            "$ref":"#/$defs/n"}"#).unwrap();
    let c = compile(&s, &host());   // must return, not recurse forever
    assert!(has_cyclic(&c.root), "a self-referential $ref compiles to Cyclic");
}

#[test]
fn an_uncompilable_pattern_still_projects_a_complete_form() {
    let s = serde_json::json!({"properties":{"a":{"type":"string","pattern":"("}}});
    let c = compile(&s, &host());
    assert_eq!(child_keys(&c.root), ["a"]);
    assert!(c.state.validatable.is_err(), "D8: the document loses validation, not its form");
}
```

- [ ] **Step 2: Run them, watch them fail.**
- [ ] **Step 3: Implement `compile.rs`:** walk with a `Vec<String>` of visited schema pointers to
      cut cycles into `FormNode::Cyclic`; merge `allOf` members left to right; classify per design
      §4 step 4, mapping every v0.1-excluded kind (`prefixItems`, object `additionalProperties`,
      `oneOf`, scalar `anyOf`) to `FormNode::Unknown` with a **Notice** naming the tier that
      implements it; sort children by `(order.unwrap_or(index), demoted)`. `constraint.rs` extracts
      the renderable subset — bounds, `multipleOf`, length, `pattern`, `uniqueItems` — as guidance
      only, never as a gate.
- [ ] **Step 4: Run them, watch them pass.**
- [ ] **Step 5: Add `insta` snapshots** for one real published Schema
      (`tests/fixtures/eslintrc.json`, checked in) and review the snapshot by eye once.
- [ ] **Step 6: Commit.** `feat(form): compile a Schema into a Form IR tree`.

---

### Task 7: Overlay — Presence, Occupancy, locked, Violation attribution

**Files:**
- Create: `crates/confyg-form/src/overlay.rs`, `crates/confyg-form/tests/overlay.rs`
- Modify: `crates/confyg-form/src/compile.rs`

**Interfaces:**
- Produces:
  `pub fn project(schema: &Value, doc: Option<&AnyDocument>, host: &HostProfile) -> Compiled`,
  the whole of design §4. `compile` becomes `project(schema, None, host)`.

- [ ] **Step 1: Write the failing test** — `tests/overlay.rs`:

```rust
#[test]
fn three_presence_states_and_the_default_stays_unwritten() {
    let s = serde_json::json!({"properties":{
        "host":{"type":"string"},
        "level":{"type":"string","enum":["info","debug"],"default":"info"},
        "port":{"type":"integer"}}});
    let doc = parse_toml("host = \"a\"\nport = \"nope\"\n");
    let c = project(&s, Some(&doc), &host());
    assert!(matches!(presence(&c.root, "host"), Presence::Set { .. }));
    assert!(matches!(presence(&c.root, "level"),
        Presence::Absent { default: Some(_), .. }), "an unwritten default is Absent, not Set");
    assert!(matches!(presence(&c.root, "port"), Presence::Invalid { .. }),
        "a type mismatch is a Violation, and the literal stays as authored");
}

#[test]
fn empty_and_absent_collections_differ() {
    let s = serde_json::json!({"properties":{"servers":{"type":"array","items":{"type":"string"}}}});
    assert_eq!(occ(&project(&s, Some(&parse_toml("servers = []\n")), &host()).root, "servers"),
               Occupancy::Empty);
    assert_eq!(occ(&project(&s, Some(&parse_toml("")), &host()).root, "servers"),
               Occupancy::Absent);
}

#[test]
fn a_container_keyword_violation_attaches_to_the_container() {
    let s = serde_json::json!({"properties":{
        "servers":{"type":"array","items":{"type":"string"},"minItems":2}}});
    let c = project(&s, Some(&parse_toml("servers = [\"a\"]\n")), &host());
    assert_eq!(violations(&c.root, "servers").len(), 1,
        "minItems reports the container's path (upstream.md Schema validation)");
}

#[test]
fn a_yaml_alias_is_locked() {
    let s = serde_json::json!({"properties":{"b":{"type":"object",
        "properties":{"host":{"type":"string"}}}}});
    let doc = parse_yaml("a: &x\n  host: h\nb: *x\n");
    assert!(locked(&project(&s, Some(&doc), &host()).root, "b").is_some(),
        "an alias renders its resolved value with no write affordance (D/B21)");
}

#[test]
fn an_unresolvable_pointer_attaches_to_the_root() { /* assert root violations == 1 */ }
```

- [ ] **Step 2: Run it, watch it fail.**
- [ ] **Step 3: Implement.** Build the value bridge and `PointerMap` with
      `schema::value_bridge::bridge`, validate with `schema::validate`, then attach each
      `Violation` by exact **Path**, falling back up the ancestor chain and finally to the root —
      the two attribution rules in design §4 step 6 are consequences of `PointerMap::resolve`
      walking up, so they are *asserted*, not re-implemented. `Presence::Invalid` is chosen when a
      **Field**'s own violations are non-empty **or** its literal does not parse as its **Widget**'s
      type, which is what activates **Raw literal fallback** (A23).
- [ ] **Step 4: Run it, watch it pass.**
- [ ] **Step 5: Snapshot one fixture per Doc format** — TOML, JSON, YAML of the same logical
      document must produce the same IR modulo literals. This is verification item 1.
- [ ] **Step 6: Commit.** `feat(form): overlay the Document onto the Form IR`.

---

### Task 8: Unknown sweep, the additionalProperties table, and the violation summary

**Files:**
- Create: `crates/confyg-form/src/unknown.rs`, `crates/confyg-form/tests/unknown.rs`
- Modify: `crates/confyg-form/src/overlay.rs`

**Interfaces:**
- Produces: `pub fn sweep(node: &mut FormNode, doc: &AnyDocument, facts: &SchemaFacts)` and
  `pub fn summary(root: &FormNode, state: &SchemaState) -> Summary` where
  `Summary { items: Vec<SummaryItem>, validation: Validation }` and
  `Validation::{Available, Unavailable { keyword: String, pointer: String }}`.

- [ ] **Step 1: Write the failing test:**

```rust
#[test]
fn an_extra_key_under_open_additional_properties_is_a_notice_not_a_violation() {
    let s = serde_json::json!({"properties":{"a":{"type":"string"}}});
    let c = project(&s, Some(&parse_toml("a = \"x\"\nz = 1\n")), &host());
    assert_eq!(unknown_keys(&c.root), ["z"]);
    assert!(summary(&c.root, &c.state).items.is_empty(),
        "confyg does not fabricate failures (design §7 B18)");
    assert_eq!(c.notices.len(), 1);
}

#[test]
fn closed_additional_properties_keeps_the_validator_s_container_violation() {
    let s = serde_json::json!({"additionalProperties": false,
                               "properties":{"a":{"type":"string"}}});
    let c = project(&s, Some(&parse_toml("a = \"x\"\nz = 1\n")), &host());
    assert_eq!(root_violations(&c.root).len(), 1,
        "the message names the key; the Violation belongs to the container");
    assert!(unknown_notice_names_key(&c, "z"));
}

#[test]
fn a_broken_pattern_says_unavailable_never_no_problems() {
    let s = serde_json::json!({"properties":{"a":{"type":"string","pattern":"("}}});
    let c = project(&s, Some(&parse_toml("a = \"x\"\n")), &host());
    assert!(matches!(summary(&c.root, &c.state).validation,
        Validation::Unavailable { .. }), "C6/D8");
}
```

- [ ] **Step 2: Run it, watch it fail.**
- [ ] **Step 3: Implement** the sweep as one pass per container, collecting every **Document** key
      with no **Form node** into a single trailing `FormNode::Unknown` per parent, and the summary
      as a depth-first walk. `Validation::Unavailable` carries the keyword and pointer the
      `Validator::new` error reported.
- [ ] **Step 4: Run it, watch it pass.**
- [ ] **Step 5: Commit.** `feat(form): sweep unknown keys and summarize violations`.

---

### Task 9: Ordinal conversion and the root slot

**Files:**
- Create: `crates/confyg-session/Cargo.toml`, `crates/confyg-session/src/lib.rs`,
  `crates/confyg-session/src/ordinal.rs`, `crates/confyg-session/tests/ordinal.rs`
- Modify: `Cargo.toml` (workspace members)

**Interfaces:**
- Produces:
  `pub fn child_ordinal(doc: &AnyDocument, parent: &Path, projection_index: usize) -> usize` and
  `pub fn schema_slot(doc: &AnyDocument, parent: &Path, key: &str, schema_order: &[String]) -> usize`.
  Both return the ordinal a `Target` wants. This is D7 and D1, the two hazards whose failure mode
  is misplaced text rather than an error, so they get their own task and their own tests before any
  mutation exists.

- [ ] **Step 1: Write the failing test** — comment-interleaved by construction:

```rust
#[test]
fn projection_index_is_not_the_target_index() {
    // Three comments before the first entry: projection 0 is child ordinal 3.
    let doc = parse_toml("# a\n# b\n# c\nx = 1\ny = 2\n");
    assert_eq!(child_ordinal(&doc, &vec![], 0), 3, "Target.index counts comments (D7)");
    assert_eq!(child_ordinal(&doc, &vec![], 1), 4);
}

#[test]
fn a_missing_key_lands_at_its_schema_position_among_present_siblings() {
    let doc = parse_toml("host = \"h\"\nca_cert = \"c\"\n");
    let order = ["host", "port", "ca_cert"].map(String::from).to_vec();
    assert_eq!(schema_slot(&doc, &vec![], "port", &order), 1, "never appended blindly (design §8)");
}

#[test]
fn a_scalar_never_lands_after_a_sub_table_in_toml() {
    let doc = parse_toml("host = \"h\"\n[tls]\non = true\n");
    let order = ["host", "tls", "port"].map(String::from).to_vec();
    assert_eq!(schema_slot(&doc, &vec![], "port", &order), 1,
        "legality wins over Schema order at the root, which does not clamp (D1)");
}
```

- [ ] **Step 2: Run it, watch it fail.**
- [ ] **Step 3: Implement.** Port `session::insertion::true_sibling_index` (it is `pub(crate)`
      upstream; `upstream.md` *The upstream bill* item 2 requests it as `model::insert_ordinal`) and
      compute the root partition from the public `Node` / `NodeKind` / `Format`, because
      `check_partition` is also `pub(crate)`. Add a `// PORTED:` comment naming the upstream path
      in both places so the future upstream helper can delete them.
- [ ] **Step 4: Run it, watch it pass.** Repeat every case for YAML, which additionally subtracts
      `root_prefix_offset`, and for JSON.
- [ ] **Step 5: Commit.** `feat(session): convert projection indices to child ordinals`.

---

### Task 10: SetValue and Unset — the round-trip matrix begins

**Files:**
- Create: `crates/confyg-session/src/lower.rs`, `crates/confyg-session/tests/roundtrip.rs`

**Interfaces:**
- Consumes: `ordinal::{child_ordinal, schema_slot}`, `confyg_form::project`.
- Produces: `pub enum SetterIntent { SetValue { path: Path, value: Value }, Unset { path: Path }, AddRepeatItem { path: Path }, RemoveRepeatItem { path: Path, index: usize }, ToggleGroup { path: Path, enable: bool }, GenerateTemplate { target: DocFormat, strategy: TemplateStrategy, comments: CommentPolicy } }`
  and `pub fn lower(intent: &SetterIntent, ir: &FormNode, doc: &AnyDocument) -> Result<Vec<Mutation>, Refused>`.
  `Refused` is only ever returned for an ungated intent the host should not have offered — it is a
  bug signal, not a user-facing path (design §5).

- [ ] **Step 1: Write the failing test** — `tests/roundtrip.rs`, the shape verification item 2
      mandates: `(schema, document, [intent]) → expected bytes`, one case per format.

```rust
#[test]
fn set_value_replaces_and_leaves_every_other_byte_alone() {
    let s = schema_host_port();
    let src = "# lead\nhost = \"a\"  # trail\nport = 80\n";
    let out = apply_all(&s, src, DocFormat::Toml,
                        &[SetterIntent::SetValue { path: key("port"), value: json!(8080) }]);
    assert_eq!(out, "# lead\nhost = \"a\"  # trail\nport = 8080\n");
}

#[test]
fn setting_a_value_equal_to_the_default_deletes_the_key() {
    let s = json!({"properties":{"level":{"type":"string","default":"info"}}});
    let out = apply_all(&s, "level = \"debug\"\n", DocFormat::Toml,
                        &[SetterIntent::SetValue { path: key("level"), value: json!("info") }]);
    assert_eq!(out, "", "Minimal write is a correctness rule (ADR 0003)");
}

#[test]
fn an_invalid_value_is_written_and_warned_about_never_refused() {
    let s = json!({"properties":{"port":{"type":"integer","minimum":1}}});
    let out = apply_all(&s, "port = 80\n", DocFormat::Toml,
                        &[SetterIntent::SetValue { path: key("port"), value: json!(-5) }]);
    assert_eq!(out, "port = -5\n", "Soft constraint");
    assert_eq!(violations_after(&s, &out).len(), 1);
}

#[test]
fn unset_is_not_offered_on_a_required_field() {
    let s = json!({"required":["host"],"properties":{"host":{"type":"string"}}});
    assert!(lower_err(&s, "host = \"a\"\n", SetterIntent::Unset { path: key("host") }).is_some());
}

#[test]
fn a_missing_key_is_inserted_at_its_schema_position() {
    let s = json!({"properties":{"host":{"type":"string"},"port":{"type":"integer"},
                                 "ca":{"type":"string"}}});
    let out = apply_all(&s, "host = \"a\"\nca = \"c\"\n", DocFormat::Toml,
                        &[SetterIntent::SetValue { path: key("port"), value: json!(80) }]);
    assert_eq!(out, "host = \"a\"\nport = 80\nca = \"c\"\n");
}
```

- [ ] **Step 2: Run them, watch them fail.**
- [ ] **Step 3: Implement `lower`** for `SetValue` and `Unset`: `Replace` when the **Field** is
      `Set` or `Invalid`; `Insert` at `schema_slot` converted through `child_ordinal` when it is
      `Absent`; `Delete` when the value equals the effective default. Every `Insert` passes
      `OnCollision::Cancel` and `suggested_key: Some(key)`.
- [ ] **Step 4: Run them, watch them pass.** Then parameterize the whole test module over
      `[Toml, Json, Yaml]` with a macro, so each case runs three times. The comment-interleaved
      variant is mandatory in every format (verification item 2).
- [ ] **Step 5: Commit.** `feat(session): lower SetValue and Unset onto confy-core mutations`.

---

### Task 11: Repeat groups, absent-parent lowering, and ToggleGroup

**Files:**
- Modify: `crates/confyg-session/src/lower.rs`, `crates/confyg-session/tests/roundtrip.rs`
- Create: `crates/confyg-session/src/fragment.rs`

**Interfaces:**
- Produces: `pub fn fragment(schema: &Value, ptr: &TemplateRef, fmt: DocFormat, style: Emission) -> String`
  with `Emission::{Headerless, HeaderBearing}` — the **Emission style** distinction TOML expresses
  as a `Target` choice rather than as fragment text (design §8).

- [ ] **Step 1: Write the failing test** — the three mandatory cases:

```rust
#[test]
fn the_first_item_of_an_absent_collection_is_a_different_mutation_from_the_second() {
    let s = json!({"properties":{"servers":{"type":"array","maxItems":2,
        "items":{"type":"object","properties":{"host":{"type":"string"}}}}}});
    let after_first = apply_all(&s, "", DocFormat::Toml,
                                &[SetterIntent::AddRepeatItem { path: key("servers") }]);
    assert_eq!(after_first, "[[servers]]\nhost = \"\"\n",
        "Absent-parent lowering: header-bearing fragment into the parent");
    let after_second = apply_all(&s, &after_first, DocFormat::Toml,
                                 &[SetterIntent::AddRepeatItem { path: key("servers") }]);
    assert_eq!(after_second, "[[servers]]\nhost = \"\"\n\n[[servers]]\nhost = \"\"\n",
        "D2: addressing the collection's Path makes the engine synthesize the header");
}

#[test]
fn add_is_not_offered_at_max_items() {
    let s = json!({"properties":{"a":{"type":"array","maxItems":1,"items":{"type":"string"}}}});
    assert!(lower_err(&s, "a = [\"x\"]\n", SetterIntent::AddRepeatItem { path: key("a") }).is_some());
}

#[test]
fn toggling_an_optional_group_writes_its_template_and_removes_the_whole_section() {
    let s = json!({"properties":{"tls":{"type":"object",
        "properties":{"on":{"type":"boolean","default":false},"ca":{"type":"string"}}}}});
    let on = apply_all(&s, "host = \"a\"\n", DocFormat::Toml,
                       &[SetterIntent::ToggleGroup { path: key("tls"), enable: true }]);
    assert_eq!(on, "host = \"a\"\n\n[tls]\nca = \"\"\n",
        "the default-valued `on` is not written (Minimal write)");
    let off = apply_all(&s, &on, DocFormat::Toml,
                        &[SetterIntent::ToggleGroup { path: key("tls"), enable: false }]);
    assert_eq!(off, "host = \"a\"\n");
}

#[test]
fn a_header_fragment_at_the_collection_path_is_rejected_upstream() {
    // Guards the D2 asymmetry: assert the Emission choice, not the engine's tolerance.
    let f = fragment(&servers_schema(), &TemplateRef("#/properties/servers/items".into()),
                     DocFormat::Toml, Emission::Headerless);
    assert!(!f.starts_with("[["));
}
```

- [ ] **Step 2: Run them, watch them fail.**
- [ ] **Step 3: Implement.** `AddRepeatItem` chooses its lowering on the collection's
      **Occupancy**: `Absent` → header-bearing fragment addressed at the parent through
      `schema_slot`; `Empty` / `Populated` → headerless fragment addressed at the collection's
      **Path**. `ToggleGroup` uses the same two lowerings. `RemoveRepeatItem` is `Delete`, gated on
      `len > minItems`. Templates render only non-default values, so **Minimal write** holds for
      generated text too.
- [ ] **Step 4: Run them, watch them pass** across all three formats.
- [ ] **Step 5: Commit.** `feat(session): add and remove Repeat items and toggle optional Groups`.

---

### Task 12: Template generation, Comment policy, and the Schema hint

**Files:**
- Create: `crates/confyg-session/src/template.rs`, `crates/confyg-session/tests/template.rs`

**Interfaces:**
- Produces: `pub enum TemplateStrategy { RequiredOnly, WithDefaults, Everything }`,
  `pub enum CommentPolicy { Allow, Deny }`,
  `pub fn comment_policy(path: Option<&str>, fmt: DocFormat, had_comments: bool) -> CommentPolicy`,
  `pub fn generate(schema: &Value, ir: &FormNode, target: DocFormat, strategy: TemplateStrategy, comments: CommentPolicy) -> String`.

- [ ] **Step 1: Write the failing test:**

```rust
#[test]
fn comment_policy_is_derived_never_chosen() {
    assert!(matches!(comment_policy(Some("a.toml"), DocFormat::Toml, false), CommentPolicy::Allow));
    assert!(matches!(comment_policy(Some("a.json"), DocFormat::Json, false), CommentPolicy::Deny));
    assert!(matches!(comment_policy(Some("a.jsonc"), DocFormat::Json, false), CommentPolicy::Allow));
    assert!(matches!(comment_policy(Some("a.json"), DocFormat::Json, true), CommentPolicy::Allow),
        "a .json that arrived with comments has a consumer that tolerates them");
    assert!(matches!(comment_policy(None, DocFormat::Json, false), CommentPolicy::Deny),
        "a new file with no extension denies until the format dialog sets one");
}

#[test]
fn a_template_carries_titles_as_comments_and_the_schema_hint() {
    let s = json!({"$id":"https://example.com/a.schema.json","required":["host"],
        "properties":{"host":{"type":"string","title":"Hostname","description":"DNS name"}}});
    let out = generate(&s, &ir_of(&s), DocFormat::Toml,
                       TemplateStrategy::RequiredOnly, CommentPolicy::Allow);
    assert_eq!(out, "#:schema https://example.com/a.schema.json\n\n\
                     # Hostname\n# DNS name\nhost = \"\"\n");
}

#[test]
fn a_strict_json_template_has_no_comments_at_all() {
    let out = generate(&titled_schema(), &ir(), DocFormat::Json,
                       TemplateStrategy::RequiredOnly, CommentPolicy::Deny);
    assert!(!out.contains("//"), "D4");
    assert!(serde_json::from_str::<serde_json::Value>(&out).is_ok());
}
```

- [ ] **Step 2: Run them, watch them fail.**
- [ ] **Step 3: Implement.** Emit the hint in each format's convention — `#:schema <url>` for TOML,
      `# yaml-language-server: $schema=<url>` for YAML, `"$schema"` for JSON — and verify the
      generated text round-trips through `detect_hint`. Comments are emitted **only** here; an
      existing **Document**'s comments are never touched.
- [ ] **Step 4: Run them, watch them pass.**
- [ ] **Step 5: Add the reverse test:** `detect_hint(generate(...)) == Some(the $id)` for all three
      formats. C3's real acceptance is that the file confyg wrote resolves its own Schema on reopen.
- [ ] **Step 6: Commit.** `feat(session): generate Templates under a derived Comment policy`.

---

### Task 13: Session state, dispatch, snapshot, and undo/redo

**Files:**
- Create: `crates/confyg-session/src/session.rs`, `crates/confyg-session/tests/session.rs`

**Interfaces:**
- Produces:
  `pub struct Session { … }` with
  `pub fn dispatch(&mut self, req: Request) -> SetterSnapshot`,
  `pub enum Request { Intent(SetterIntent), Command(SessionCommand) }`,
  `pub enum SessionCommand { Open { text: String, fmt: DocFormat, path: Option<String> }, Save, ConvertFormat(DocFormat), LoadSchema { source: SchemaSource, text: String }, Undo, Redo }`,
  `pub struct SetterSnapshot { ir: FormNode, summary: Summary, text: String, notices: Vec<Notice>, fetch: Option<SchemaFetchRequest>, can_undo: bool, can_redo: bool }`.
  `confyg-ffi` and `web/` consume exactly this type and nothing else.

- [ ] **Step 1: Write the failing test:**

```rust
#[test]
fn opening_without_a_schema_emits_a_fetch_request_from_the_hint() {
    let mut s = Session::new();
    let snap = s.dispatch(Request::Command(SessionCommand::Open {
        text: "#:schema https://example.com/a.json\nhost = \"a\"\n".into(),
        fmt: DocFormat::Toml, path: Some("a.toml".into()) }));
    assert!(snap.fetch.is_some(), "confyg-session performs no I/O (design §6)");
    assert!(matches!(snap.ir, FormNode::Unknown { .. } | FormNode::Group { .. }));
}

#[test]
fn with_no_schema_at_all_the_document_is_one_unknown_group() {
    let mut s = Session::new();
    let snap = s.dispatch(Request::Command(SessionCommand::Open {
        text: "host = \"a\"\n".into(), fmt: DocFormat::Toml, path: None }));
    assert!(matches!(snap.ir, FormNode::Unknown { .. }), "design §6 step 4");
}

#[test]
fn undo_is_one_entry_per_committed_intent() {
    let mut s = session_with(schema_host_port(), "port = 80\n");
    s.dispatch(intent(SetterIntent::SetValue { path: key("port"), value: json!(1) }));
    s.dispatch(intent(SetterIntent::SetValue { path: key("port"), value: json!(2) }));
    assert_eq!(s.dispatch(cmd(SessionCommand::Undo)).text, "port = 1\n");
    assert_eq!(s.dispatch(cmd(SessionCommand::Undo)).text, "port = 80\n");
    assert_eq!(s.dispatch(cmd(SessionCommand::Redo)).text, "port = 1\n");
}

#[test]
fn convert_format_re_emits_the_whole_document() {
    let mut s = session_with(schema_host_port(), "host = \"a\"\n");
    let out = s.dispatch(cmd(SessionCommand::ConvertFormat(DocFormat::Json))).text;
    assert_eq!(serde_json::from_str::<serde_json::Value>(&out).unwrap()["host"], "a");
}
```

- [ ] **Step 2: Run them, watch them fail.**
- [ ] **Step 3: Implement.** Port confy's `session::undo_redo` full-text snapshot ring (69 lines,
      `upstream.md`), push one entry per committed intent, and recompile the IR after every
      dispatch — §4 is cheap and `apply` already returns the new text. `ConvertFormat` calls
      `model::convert::convert`; `ConvertKind` is never used (design §1).
- [ ] **Step 4: Run them, watch them pass.**
- [ ] **Step 5: Commit.** `feat(session): dispatch intents and commands behind one snapshot type`.

---

### Task 14: The intent postcondition guard

**Files:**
- Create: `crates/confyg-session/tests/postcondition.rs`
- Modify: `crates/confyg-session/src/session.rs`

**Interfaces:**
- Produces: `Session::dispatch` gains a debug-assertion-grade check:
  `pub fn predicted(intent: &SetterIntent, before: &FormNode) -> Prediction` and a comparison
  against the recompiled IR. A mismatch is a **Notice** carrying both shapes in release builds and
  a panic in tests — this is the D9 guard, whose failure mode upstream is *success plus a
  structurally different document*.

- [ ] **Step 1: Write the failing test:**

```rust
#[test]
fn every_intent_recompiles_to_the_shape_it_predicted() {
    for (schema, src, intent) in every_v0_1_intent_case() {
        let mut s = session_with(schema.clone(), src);
        let snap = s.dispatch(intent(intent.clone()));
        assert_eq!(predicted(&intent, &before_ir(&schema, src)).presence_at(intent.path()),
                   actual_presence(&snap.ir, intent.path()),
                   "D9: {intent:?} wrote a shape it did not predict");
    }
}

#[test]
fn a_generic_placeholder_key_never_appears() {
    for (schema, src, intent) in every_v0_1_intent_case() {
        let mut s = session_with(schema, src);
        assert!(!s.dispatch(intent(intent)).text.contains("placeholder"),
            "every Insert passes an explicit suggested_key (upstream.md Fragment contract)");
    }
}
```

- [ ] **Step 2: Run them, watch them fail.**
- [ ] **Step 3: Implement `predicted`** for the six v0.1 intents: `SetValue` → `Set` (or `Absent`
      when it lowered to `Delete`), `Unset` → `Absent`, `AddRepeatItem` → `Populated` with
      `len + 1`, `RemoveRepeatItem` → `len - 1`, `ToggleGroup` → `Populated` / `Absent`.
- [ ] **Step 4: Run them, watch them pass.**
- [ ] **Step 5: Commit.** `feat(session): assert every intent's postcondition against the recompile`.

---

### Task 15: Write-neutrality as a property test

**Files:**
- Create: `crates/confyg-session/tests/write_neutrality.rs`

**Interfaces:**
- Consumes: everything. Produces no API — this is verification item 6, and it is the test that
  makes ADR 0004's invariant real rather than a review guideline.

- [ ] **Step 1: Write the failing test:**

```rust
fn host_profiles() -> Vec<HostProfile> { /* full, no-mask, no-slide, no-filter, TUI-ish */ }
fn presentations() -> Vec<serde_json::Value> {
    // The same schema decorated with every vocabulary member that could plausibly leak.
    vec![json!({}),
         json!({"affordance":"text"}), json!({"order":9}), json!({"unit":"MiB"}),
         json!({"collapsed":true}), json!({"demoted":true}),
         json!({"label":"X"}), json!({"help":"Y"}),
         json!({"optionLabels":{"info":"Informational"}})]
}

#[test]
fn presentation_can_never_reach_the_bytes() {
    for fmt in [DocFormat::Toml, DocFormat::Json, DocFormat::Yaml] {
        for case in every_v0_1_intent_case_for(fmt) {
            let baseline = run(&case, &json!({}), &host_profiles()[0]);
            for p in presentations() {
                for h in host_profiles() {
                    assert_eq!(run(&case, &p, &h), baseline,
                        "Write-neutrality broken by {p} on {h:?} (ADR 0004)");
                }
            }
        }
    }
}
```

- [ ] **Step 2: Run it.** It may pass immediately. That is fine and expected: this test's job is to
      fail the day someone adds a presentation feature that writes. **Do not weaken it if it fails
      — fix the leak.** A leak found here is a design violation, not a test bug.
- [ ] **Step 3: Add it to CI** as a required job, named `write-neutrality`, so the invariant has a
      visible gate.
- [ ] **Step 4: Commit.** `test: pin Write-neutrality as a property over presentation inputs`.

---

### Task 16: The WASM boundary

**Files:**
- Create: `crates/confyg-ffi/Cargo.toml`, `crates/confyg-ffi/src/lib.rs`,
  `crates/confyg-ffi/tests/boundary.rs`
- Modify: `Cargo.toml`, `.github/workflows/ci.yml`

**Interfaces:**
- Produces exactly one export: `pub fn dispatch(state: &mut Handle, request_json: &str) -> String`
  — a `Request` in, a `SetterSnapshot` out, both as JSON. Live `pattern` checks cross this boundary
  rather than being reimplemented host-side, so the boundary also exports
  `pub fn check(state: &Handle, path_json: &str, literal: &str) -> String` returning the
  validator's own **Violations** for a buffer that is not yet committed.

- [ ] **Step 1: Write the failing test:**

```rust
#[test]
fn the_boundary_is_json_in_json_out() {
    let mut h = Handle::new();
    let out = dispatch(&mut h, r#"{"command":{"open":{"text":"host = \"a\"\n","fmt":"toml"}}}"#);
    let snap: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert!(snap["ir"].is_object() && snap["text"].is_string());
}

#[test]
fn live_checks_use_the_validators_own_engine() {
    // fancy-regex accepts a lookahead Rust regex rejects; the form must agree with the validator.
    let mut h = handle_with(json!({"properties":{"a":{"type":"string",
        "pattern":"^(?!x)[a-z]+$"}}}), "");
    let v: serde_json::Value = serde_json::from_str(&check(&h, r#"["a"]"#, "xyz")).unwrap();
    assert_eq!(v.as_array().unwrap().len(), 1,
        "form warnings and Violations never disagree (design §7)");
}
```

- [ ] **Step 2: Run them, watch them fail.**
- [ ] **Step 3: Implement.** `Handle` owns a `Session`. Nothing but serialization lives here: the
      FFI crate holds no logic, so the TUI host at v0.3 can link `confyg-session` directly.
- [ ] **Step 4: Run them, watch them pass;** then `wasm-pack build crates/confyg-ffi --target web`
      and add that build to CI.
- [ ] **Step 5: Commit.** `feat(ffi): expose dispatch and live checks over one WASM boundary`.

---

## Phase B — the web host

### Task 17: Renderer shell, Appearance tokens, and Partition

**Files:**
- Create: `web/package.json`, `web/vite.config.ts`, `web/index.html`,
  `web/src/main.ts`, `web/src/render.ts`, `web/src/tokens.css`, `web/src/partition.ts`
- Port from confy: `web/src/host-io.ts`, `web/src/i18n.ts`, `web/src/style.css`

**Interfaces:**
- Consumes: `SetterSnapshot` JSON from Task 16.
- Produces: `render(snapshot: SetterSnapshot, root: HTMLElement): void`, and
  `partition(ir: FormNode): Partition` returning `{ kind: "scroll" | "sections", sections: … }`.

- [ ] **Step 1: Port the token set and add confyg's own roles.** Copy confy's OKLCH chrome tokens
      from `confy/web/style.css:6-69`; add `--ghost`, `--violation`, `--locked`, `--required-marker`,
      `--deprecated`, `--inherited-option` to `tokens.css`. Do **not** port confy's data-type
      colours (presentation §7).

- [ ] **Step 2: Write the failing test** — `web/src/partition.test.ts`:

```ts
test("fewer than three depth-1 Groups falls back to scroll", () => {
  expect(partition(irWithGroups(2)).kind).toBe("scroll");
  expect(partition(irWithGroups(3)).kind).toBe("sections");
});
test("sections come from depth-1 Groups only", () => {
  expect(partition(irWithGroups(4)).sections.map(s => s.key))
    .toEqual(["a", "b", "c", "d"]);
});
```

- [ ] **Step 3: Run `npm test -w web`,** watch it fail.
- [ ] **Step 4: Implement `partition.ts`** per presentation §5, and `render.ts` as a recursive IR
      walk dispatching on `kind`. `render.ts` is new code: confy's `render.ts` and `ViewRow` are
      not reused (ADR 0002).
- [ ] **Step 5: Run it, watch it pass.**
- [ ] **Step 6: Commit.** `feat(web): render the Form IR shell with confyg's Appearance tokens`.

---

### Task 18: The widget set and three-state Presence

**Files:**
- Create: `web/src/widgets/{text,menu,radio,tristate,stepper,slider,display}.ts`,
  `web/src/widgets/index.ts`, `web/src/widgets/presence.test.ts`

**Interfaces:**
- Produces: `mount(node: FieldNode, ctx: Ctx): HTMLElement` per module, and one registry in
  `index.ts` keyed by the `Widget` name — the closed vocabulary from Task 2, so an unmapped name is
  a build error rather than a blank field.

- [ ] **Step 1: Write the failing test** — `presence.test.ts`:

```ts
test("every widget offers all three Presence states", () => {
  for (const w of ALL_WIDGETS) {
    const el = mountFor(w, absentFieldWithDefault("info"));
    expect(el.textContent).toContain("info");                       // Ghost text
    expect(inheritedOption(el) ?? unsetAffordance(el)).toBeTruthy(); // how Unset is offered
  }
});
test("a boolean is three-state, never a bare checkbox", () => {
  const el = mountFor("tristate", absentBooleanWithDefault(false));
  expect(optionValues(el)).toEqual(["inherited", "true", "false"]); // ADR 0002
});
test("a locked node has no write affordance", () => {
  expect(writableControls(mountFor("text", lockedField()))).toHaveLength(0);
});
test("a clamped widget explains itself", () => {
  const el = mountFor("text", maskedFieldClampedToText());
  expect(degradeNotice(el)).toContain("masked");   // presentation §6
});
```

- [ ] **Step 2: Run it, watch it fail.**
- [ ] **Step 3: Implement each widget.** A control that can express its default renders the
      inherited option; one that cannot keeps a separate **Unset** affordance (ADR 0003).
      `filterable-menu` is never mounted in v0.1 — the clamp already turned it into `menu`.
- [ ] **Step 4: Run it, watch it pass.**
- [ ] **Step 5: Commit.** `feat(web): mount the v0.1 widget set with three-state Presence`.

---

### Task 19: Repeat cards, the violation summary, and Form search

**Files:**
- Create: `web/src/repeat.ts`, `web/src/summary.ts`, `web/src/search.ts` + colocated tests

- [ ] **Step 1: Write the failing tests:**

```ts
test("a Repeat group shows a count badge and gates + at maxItems", () => {
  const el = repeat({ items: 3, bounds: { min: 1, max: 5 } });
  expect(badge(el)).toBe("(3/5)");
  expect(addButton(repeat({ items: 5, bounds: { min: 1, max: 5 } })).disabled).toBe(true);
});
test("a card is titled from labelFrom, falling back to an ordinal", () => {
  expect(cardTitle({ labelFrom: "name", item: { name: "web-1" } })).toBe("web-1");
  expect(cardTitle({ labelFrom: "name", item: {}, title: "Server", n: 2 })).toBe("Server #2");
});
test("a scalar Repeat renders rows, not cards", () => {
  expect(repeat({ scalarItems: true }).querySelectorAll(".card")).toHaveLength(0);
});
test("an uncompilable Schema says validation unavailable, never no problems", () => {
  expect(summary({ validation: { kind: "unavailable", keyword: "pattern" } }).textContent)
    .toMatch(/unavailable/i);
});
test("Form search matches titles and descriptions, not only keys", () => {
  expect(search(ir, "timeout").map(r => r.path)).toContain("server.deadline");
});
```

- [ ] **Step 2: Run them, watch them fail.**
- [ ] **Step 3: Implement.** `search.ts` matches over title, description, and key; it shares no code
      and no term with the **Option filter** (presentation §5.3). Summary items jump to their node.
- [ ] **Step 4: Run them, watch them pass.**
- [ ] **Step 5: Commit.** `feat(web): render Repeat cards, the violation summary, and Form search`.

---

### Task 20: The real-binary check

**Files:**
- Create: `tests/e2e/first-run.spec.ts`, `docs/reference/README.md` subsystem rows
- Modify: `README.md`, `CHANGELOG.md`, `docs/spec/*` `Status:` lines

- [ ] **Step 1: Build the real thing.** `wasm-pack build crates/confyg-ffi --target web && npm run build -w web`.

- [ ] **Step 2: Write the end-to-end test** against a real published Schema (the `eslintrc` fixture
      from Task 6), exercising verification item 5's exact flow:

```ts
test("open → add a Repeat item → set values → Unset one → save", async ({ page }) => {
  await page.goto(BUILD_URL);
  await openFixture(page, "eslintrc.json");
  await page.getByRole("button", { name: /add/i }).first().click();
  await page.getByLabel("host").fill("web-1");
  await page.getByRole("button", { name: /unset/i }).first().click();
  const saved = await savedText(page);
  expect(saved).toContain("web-1");
  expect(saved).not.toContain("placeholder");
});
```

- [ ] **Step 3: Run it against the built artifact, not a dev server.** Green unit tests are not
      evidence that the flow works (design §11 item 5).

- [ ] **Step 4: Reproduce the two hazards by hand on the real build** and record the result in
      `docs/debug/` if either misbehaves: insert a key into a TOML table that already has a
      sub-table (D1), and set a value in a file with a comment between every sibling (D7).

- [ ] **Step 5: Write the reference docs.** `docs/reference/` gains one contract file per
      subsystem — `form-ir.md`, `intents.md`, `presentation.md` — describing *current behavior*.
      Then flip both specs' `Status:` to `Superseded` where reference now owns the text, per
      `CONTEXT.md`'s lifecycle table, and move this plan to `docs/plan/README.md`'s Landed table.

- [ ] **Step 6: Commit.** `feat: confyg v0.1`.

---

## Self-review

**Spec coverage.** design §3 → Tasks 2, 6, 7; §4 steps 1–7 → Tasks 6, 7, 8 (step 0 is C11, v0.2);
§5 intents → Tasks 10–13 (the seven v0.2/v0.3 intents are excluded by the Global Constraints);
§5 commands → Task 13; §6 → Task 13 (catalog is v0.2); §7 A1/A2/A5/A7/A8/A9/A21/A23 → Tasks 5, 18;
A3/A4/A6/A10–A20/A22 are v0.2+; B18 → Task 8; B21 → Task 7; C1/C3/C4/C5/C6 → Tasks 12, 8, 10, 13,
19; D1–D9 → Tasks 9, 11, 7, 12, 6, 3, 9, 8, 14; §8 → Tasks 9–12; §11 items 1–6 → Tasks 6/7, 10/11,
7/18, 14, 20, 15. presentation §1 → Task 15; §2 → Task 4; §3–§4 → Task 5; §5 → Tasks 17, 19;
§6 → Task 18; §7 → Task 17.

**Gaps accepted deliberately.** `Presence::Absent { remarked }` is carried in the type from Task 2
but never produced in v0.1 — `Remark` is a v0.2 intent, and threading the field later would be a
breaking IR change for `web/`. `Density` is in `HostProfile` from Task 5 but has only
`Density::Desktop` wired, because its mapping is an open question (presentation §10); the enum
exists so the clamp signature does not change when it is answered.

**Type consistency.** `HostProfile` (Task 5) is consumed unchanged by `compile`/`project`
(Tasks 6, 7) and `Handle` (Task 16). `SetterIntent` (Task 10) is extended, never renamed, in
Tasks 11 and 12. `SetterSnapshot` (Task 13) is the only type crossing the FFI boundary, and Phase B
consumes exactly its fields. `Widget` (Task 2) is the single key space for `ladder` (Task 5) and the
web registry (Task 18).

**Ordering.** Tasks 9, 14, and 15 are deliberately early relative to their apparent value: D7 and D9
are the two hazards that report success while writing the wrong bytes, and Write-neutrality is
cheapest to enforce before there is anything to leak.
