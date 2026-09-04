//! The Form IR: what a host renders. Design §3.
//!
//! Serialization is stable and snapshot-friendly — `web/` and `insta` both read it — so every
//! sum type is externally tagged on `kind` and every name is camelCase.

use confy_core::model::node::Path;
use confy_core::schema::types::Violation;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// One node of the projected form.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum FormNode {
    Field {
        path: Path,
        widget: Widget,
        intended: Widget,
        presence: Presence,
        meta: FieldMeta,
    },
    Group {
        path: Path,
        meta: NodeMeta,
        children: Vec<FormNode>,
        occupancy: Occupancy,
        toggle: Option<GroupToggle>,
    },
    Repeat {
        path: Path,
        meta: NodeMeta,
        items: Vec<FormNode>,
        occupancy: Occupancy,
        bounds: Bounds,
        item_template: TemplateRef,
        label_from: Option<String>,
    },
    Unknown {
        path: Path,
        raw_preview: String,
    },
    Cyclic {
        path: Path,
        schema_ptr: String,
    },
}

/// The three states every Field renders. An unwritten default is `Absent`, never `Set`
/// (ADR 0003).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Presence {
    Absent {
        default: Option<Value>,
        /// v0.2's `Remark` intent fills this; carried from v0.1 so adding it is not a breaking
        /// IR change for hosts.
        remarked: Option<String>,
    },
    Set {
        literal: String,
    },
    Invalid {
        literal: String,
        violations: Vec<Violation>,
    },
}

/// A container's state. `Empty` and `Absent` are different facts (ADR 0003).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Occupancy {
    Absent,
    Empty,
    Populated,
}

/// The closed Widget vocabulary. Exactly the names in presentation §4's ladder table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Widget {
    Text,
    RawText,
    DisplayOnly,
    Radio,
    Menu,
    FilterableMenu,
    CheckboxSet,
    Tristate,
    Stepper,
    Slider,
    Textarea,
    Masked,
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Bounds {
    pub min: Option<usize>,
    pub max: Option<usize>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupToggle {
    pub enabled: bool,
}

/// A JSON pointer into the Schema, not inlined text, so the IR stays small.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemplateRef(pub String);

/// A node the Document renders but confyg must not offer a write affordance for.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Locked {
    pub reason: LockedReason,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LockedReason {
    YamlAlias,
    MergeKey,
}

/// The renderable subset of the Schema's constraints. Guidance only, never a gate.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Constraint {
    Minimum { value: f64, exclusive: bool },
    Maximum { value: f64, exclusive: bool },
    MultipleOf { value: f64 },
    MinLength { value: usize },
    MaxLength { value: usize },
    Pattern { source: String },
    UniqueItems,
}

/// Document-level facts about the Schema behind this form.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SchemaState {
    pub projected: bool,
    /// `Err` when `Validator::new` failed: one bad `pattern` costs the whole document its
    /// validation, and confyg models that as a state rather than as silence (D8).
    pub validatable: Result<(), SchemaCompileError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SchemaCompileError {
    pub keyword: String,
    pub pointer: String,
    pub message: String,
}
