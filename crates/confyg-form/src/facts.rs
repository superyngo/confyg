//! Schema keyword introspection.
//!
//! This is the keyword set `confy_core::schema::hints_edit` does not read — `default`,
//! `examples`, `required`, `deprecated`, `readOnly`, `writeOnly`, `prefixItems`,
//! `additionalProperties` — which is why it is confyg's (`upstream.md` *The upstream bill*).
//!
//! One pass over the object, every field defaulting. A malformed keyword reads as absent and
//! never panics, because design §4 must produce a complete form from any input.

use serde_json::Value;

/// The declared types of a node. A Schema may name several.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TypeSet(pub Vec<String>);

impl TypeSet {
    pub fn has(&self, ty: &str) -> bool {
        self.0.iter().any(|t| t == ty)
    }

    /// The single type to classify on, when there is exactly one useful answer.
    pub fn sole(&self) -> Option<&str> {
        let mut real = self.0.iter().filter(|t| *t != "null");
        match (real.next(), real.next()) {
            (Some(t), None) => Some(t.as_str()),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct NumBounds {
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub exclusive_min: Option<f64>,
    pub exclusive_max: Option<f64>,
}

/// Length bounds, shared by `minLength`/`maxLength` and `minItems`/`maxItems`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LenBounds {
    pub min: Option<usize>,
    pub max: Option<usize>,
}

/// Design §7's three-form `additionalProperties` table.
#[derive(Debug, Clone, PartialEq)]
pub enum AdditionalProperties {
    Schema(Value),
    Open,
    Closed,
}

#[derive(Debug, Clone, Default)]
pub struct SchemaFacts {
    pub ty: Option<TypeSet>,
    pub default: Option<Value>,
    pub examples: Vec<Value>,
    pub enum_values: Option<Vec<Value>>,
    pub const_value: Option<Value>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub deprecated: bool,
    pub read_only: bool,
    pub write_only: bool,
    pub format: Option<String>,
    pub bounds: NumBounds,
    pub len: LenBounds,
    pub pattern: Option<String>,
    pub multiple_of: Option<f64>,
    pub unique_items: bool,
    pub additional: AdditionalProperties,
    pub required: Vec<String>,
    pub prefix_items: Option<Vec<Value>>,
}

impl Default for AdditionalProperties {
    fn default() -> Self {
        AdditionalProperties::Open
    }
}

fn str_at(schema: &Value, key: &str) -> Option<String> {
    schema.get(key)?.as_str().map(str::to_owned)
}

fn num_at(schema: &Value, key: &str) -> Option<f64> {
    schema.get(key)?.as_f64()
}

fn usize_at(schema: &Value, key: &str) -> Option<usize> {
    schema.get(key)?.as_u64().map(|n| n as usize)
}

fn flag_at(schema: &Value, key: &str) -> bool {
    schema.get(key).and_then(Value::as_bool).unwrap_or(false)
}

fn array_at(schema: &Value, key: &str) -> Option<Vec<Value>> {
    Some(schema.get(key)?.as_array()?.clone())
}

fn type_set(schema: &Value) -> Option<TypeSet> {
    match schema.get("type")? {
        Value::String(s) => Some(TypeSet(vec![s.clone()])),
        Value::Array(items) => {
            let names: Vec<String> = items
                .iter()
                .filter_map(|v| v.as_str().map(str::to_owned))
                .collect();
            (!names.is_empty()).then_some(TypeSet(names))
        }
        _ => None,
    }
}

fn additional(schema: &Value) -> AdditionalProperties {
    match schema.get("additionalProperties") {
        None => AdditionalProperties::Open,
        Some(Value::Bool(false)) => AdditionalProperties::Closed,
        Some(Value::Bool(true)) => AdditionalProperties::Open,
        Some(other) => AdditionalProperties::Schema(other.clone()),
    }
}

/// Read every keyword confyg cares about out of one Schema object.
pub fn facts(schema: &Value) -> SchemaFacts {
    SchemaFacts {
        ty: type_set(schema),
        default: schema.get("default").cloned(),
        examples: array_at(schema, "examples").unwrap_or_default(),
        enum_values: array_at(schema, "enum"),
        const_value: schema.get("const").cloned(),
        title: str_at(schema, "title"),
        description: str_at(schema, "description"),
        deprecated: flag_at(schema, "deprecated"),
        read_only: flag_at(schema, "readOnly"),
        write_only: flag_at(schema, "writeOnly"),
        format: str_at(schema, "format"),
        bounds: NumBounds {
            min: num_at(schema, "minimum"),
            max: num_at(schema, "maximum"),
            exclusive_min: num_at(schema, "exclusiveMinimum"),
            exclusive_max: num_at(schema, "exclusiveMaximum"),
        },
        len: LenBounds {
            // A node is a string or a collection, never both, so the two keyword pairs share
            // one slot rather than forcing every caller to pick.
            min: usize_at(schema, "minLength").or_else(|| usize_at(schema, "minItems")),
            max: usize_at(schema, "maxLength").or_else(|| usize_at(schema, "maxItems")),
        },
        pattern: str_at(schema, "pattern"),
        multiple_of: num_at(schema, "multipleOf"),
        unique_items: flag_at(schema, "uniqueItems"),
        additional: additional(schema),
        required: array_at(schema, "required")
            .unwrap_or_default()
            .iter()
            .filter_map(|v| v.as_str().map(str::to_owned))
            .collect(),
        prefix_items: array_at(schema, "prefixItems"),
    }
}
