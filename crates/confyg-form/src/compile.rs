//! Design §4 steps 1–5: resolve, merge, classify, order.
//!
//! Document overlay is `overlay.rs`; this module produces the all-**Absent** tree. It needs no
//! validator, so a Schema that cannot compile for validation still projects a complete form.

use serde_json::{Map, Value};

use crate::affordance::{self, HostProfile};
use crate::constraint;
use crate::facts::{self, AdditionalProperties, SchemaFacts};
use crate::ir::*;
use crate::notice::Notice;
use crate::vocab::{self, Presentation};

use confy_core::model::node::{Path, Seg};

/// A compiled form plus the document-level facts a host needs to frame it.
#[derive(Debug, Clone)]
pub struct Compiled {
    pub root: FormNode,
    pub state: SchemaState,
    pub notices: Vec<Notice>,
}

/// The last **Path** segment's key, when the node has one. Hosts and tests both walk by key.
pub fn key_of(node: &FormNode) -> Option<&str> {
    let path = path_of(node);
    match path.last() {
        Some(Seg::Key(k)) => Some(k),
        _ => None,
    }
}

pub fn path_of(node: &FormNode) -> &Path {
    match node {
        FormNode::Field { path, .. }
        | FormNode::Group { path, .. }
        | FormNode::Repeat { path, .. }
        | FormNode::Unknown { path, .. }
        | FormNode::Cyclic { path, .. } => path,
    }
}

/// The all-**Absent** tree: `project(schema, None, host)` in design §4's terms.
pub fn compile(schema: &Value, host: &HostProfile) -> Compiled {
    let mut ctx = Ctx {
        root: schema,
        host,
        notices: Vec::new(),
        visited: Vec::new(),
    };
    let root = ctx.build(schema, "#".to_owned(), Vec::new(), None, false);
    Compiled {
        root,
        state: SchemaState {
            projected: true,
            validatable: validatable(schema),
        },
        notices: ctx.notices,
    }
}

/// Whether the Schema compiles for validation at all. One uncompilable `pattern` costs the whole
/// document its validation, and confyg models that as a state rather than as silence (D8).
pub fn validatable(schema: &Value) -> Result<(), SchemaCompileError> {
    match jsonschema::Validator::new(schema) {
        Ok(_) => Ok(()),
        Err(e) => {
            let pointer = e.schema_path.to_string();
            Err(SchemaCompileError {
                keyword: pointer
                    .rsplit('/')
                    .next()
                    .unwrap_or_default()
                    .to_owned(),
                pointer,
                message: e.to_string(),
            })
        }
    }
}

struct Ctx<'a> {
    root: &'a Value,
    host: &'a HostProfile,
    notices: Vec<Notice>,
    /// `$ref` pointers on the current path. A pointer already here is a cycle (design §4 step 1).
    visited: Vec<String>,
}

impl Ctx<'_> {
    fn build(
        &mut self,
        raw: &Value,
        ptr: String,
        path: Path,
        key: Option<&str>,
        required: bool,
    ) -> FormNode {
        // Step 1: resolve.
        if let Some(reference) = raw.get("$ref").and_then(Value::as_str) {
            return self.follow(reference, path, key, required);
        }

        // Step 2: merge.
        let merged = self.merge_all_of(raw, &ptr);
        let f = facts::facts(&merged);
        let (presentation, notices) = vocab::read(&merged);
        self.notices.extend(notices);

        // Step 4: classify. Step 3 (conditionals) is v0.3.
        self.classify(&merged, &f, &presentation, ptr, path, key, required)
    }

    fn follow(&mut self, reference: &str, path: Path, key: Option<&str>, required: bool) -> FormNode {
        if !reference.starts_with('#') {
            self.notices.push(Notice::new(
                "form.compile.external-ref",
                format!("`{reference}` is an external reference; v0.2 requests it from the host"),
            ));
            return FormNode::Unknown {
                path,
                raw_preview: reference.to_owned(),
            };
        }
        if self.visited.iter().any(|v| v == reference) {
            return FormNode::Cyclic {
                path,
                schema_ptr: reference.to_owned(),
            };
        }
        let Some(target) = self.root.pointer(reference.trim_start_matches('#')) else {
            self.notices.push(Notice::new(
                "form.compile.unresolved-ref",
                format!("`{reference}` does not resolve in this Schema"),
            ));
            return FormNode::Unknown {
                path,
                raw_preview: reference.to_owned(),
            };
        };
        let target = target.clone();
        self.visited.push(reference.to_owned());
        let node = self.build(&target, reference.to_owned(), path, key, required);
        self.visited.pop();
        node
    }

    /// Flatten `allOf` into one effective subschema, left to right. A conflicting keyword keeps
    /// the narrowest constraint and records a diagnostic; it never fails compilation.
    fn merge_all_of(&mut self, raw: &Value, ptr: &str) -> Value {
        let Some(members) = raw.get("allOf").and_then(Value::as_array) else {
            return raw.clone();
        };
        let mut out: Map<String, Value> = raw
            .as_object()
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter(|(k, _)| k != "allOf")
            .collect();
        for member in members {
            let member = match member.get("$ref").and_then(Value::as_str) {
                Some(r) => match self.root.pointer(r.trim_start_matches('#')) {
                    Some(v) => v.clone(),
                    None => continue,
                },
                None => member.clone(),
            };
            let Some(fields) = member.as_object() else {
                continue;
            };
            for (k, v) in fields {
                match out.get(k) {
                    None => {
                        out.insert(k.clone(), v.clone());
                    }
                    Some(existing) if existing == v => {}
                    Some(existing) => {
                        let narrowed = narrowest(k, existing, v);
                        if narrowed.is_none() {
                            self.notices.push(Notice::new(
                                "form.compile.allof-conflict",
                                format!("`{ptr}` merges conflicting `{k}`; kept the first"),
                            ));
                        }
                        if let Some(n) = narrowed {
                            out.insert(k.clone(), n);
                        }
                    }
                }
            }
        }
        Value::Object(out)
    }

    #[allow(clippy::too_many_arguments)]
    fn classify(
        &mut self,
        merged: &Value,
        f: &SchemaFacts,
        p: &Presentation,
        ptr: String,
        path: Path,
        key: Option<&str>,
        required: bool,
    ) -> FormNode {
        let excluded = |what: &str, tier: &str| {
            Notice::new(
                "form.compile.excluded",
                format!("`{what}` projects as an Unknown group in v0.1; {tier} implements it"),
            )
        };

        if merged.get("prefixItems").is_some() {
            self.notices.push(excluded("prefixItems", "v0.2"));
            return unknown(merged, path);
        }
        if merged.get("oneOf").is_some() {
            self.notices.push(excluded("oneOf", "v0.3"));
            return unknown(merged, path);
        }
        if merged.get("anyOf").is_some() {
            self.notices.push(excluded("anyOf", "v0.3"));
            return unknown(merged, path);
        }

        if let Some(properties) = merged.get("properties").and_then(Value::as_object) {
            let mut children: Vec<(i64, bool, FormNode)> = Vec::new();
            for (index, (child_key, child_schema)) in properties.iter().enumerate() {
                let (child_presentation, _) = vocab::read(child_schema);
                let mut child_path = path.clone();
                child_path.push(Seg::Key(child_key.clone()));
                let node = self.build(
                    child_schema,
                    format!("{ptr}/properties/{child_key}"),
                    child_path,
                    Some(child_key),
                    f.required.iter().any(|r| r == child_key),
                );
                children.push((
                    child_presentation.order.unwrap_or(index as i64),
                    child_presentation.demoted.unwrap_or(false),
                    node,
                ));
            }
            // Step 5: Schema order, overridable by `x-confyg.order`, with `demoted` sinking to
            // the end. Required Fields are deliberately not hoisted.
            children.sort_by_key(|(order, demoted, _)| (*demoted, *order));
            return FormNode::Group {
                path,
                meta: node_meta(f, p, key),
                children: children.into_iter().map(|(_, _, n)| n).collect(),
                occupancy: Occupancy::Absent,
                toggle: (!required).then_some(GroupToggle { enabled: false }),
            };
        }

        let is_array = f.ty.as_ref().is_some_and(|t| t.has("array"))
            || (merged.get("items").is_some() && f.ty.is_none());
        if is_array {
            let item_ptr = TemplateRef(format!("{ptr}/items"));
            let items_schema = merged.get("items").cloned().unwrap_or(Value::Object(Map::new()));
            return FormNode::Repeat {
                path,
                meta: node_meta(f, p, key),
                items: Vec::new(),
                occupancy: Occupancy::Absent,
                bounds: Bounds {
                    min: f.len.min,
                    max: f.len.max,
                },
                label_from: p.label_from.clone().or_else(|| derive_label_from(&items_schema)),
                item_template: item_ptr,
            };
        }

        // A `additionalProperties` / `patternProperties` *schema* with no `properties` is a Map
        // group, which is v0.2.
        if matches!(f.additional, AdditionalProperties::Schema(_))
            || merged.get("patternProperties").is_some()
        {
            self.notices
                .push(excluded("additionalProperties as a schema", "v0.2"));
            return unknown(merged, path);
        }
        if f.ty.as_ref().is_some_and(|t| t.has("object")) {
            return FormNode::Group {
                path,
                meta: node_meta(f, p, key),
                children: Vec::new(),
                occupancy: Occupancy::Absent,
                toggle: (!required).then_some(GroupToggle { enabled: false }),
            };
        }

        let (widget, intended, notices) = affordance::resolve(f, p, false, self.host);
        self.notices.extend(notices);
        FormNode::Field {
            path,
            widget,
            intended,
            presence: Presence::Absent {
                default: f.default.clone(),
                remarked: None,
            },
            meta: FieldMeta {
                node: node_meta(f, p, key),
                default: f.default.clone(),
                examples: f.examples.clone(),
                required,
                read_only: f.read_only,
                write_only: f.write_only,
                unit: p.unit.clone(),
                constraints: constraint::extract(f),
                raw: false,
            },
        }
    }
}

fn unknown(merged: &Value, path: Path) -> FormNode {
    FormNode::Unknown {
        path,
        raw_preview: merged.to_string(),
    }
}

fn node_meta(f: &SchemaFacts, p: &Presentation, key: Option<&str>) -> NodeMeta {
    NodeMeta {
        title: p
            .label
            .clone()
            .or_else(|| f.title.clone())
            .or_else(|| key.map(str::to_owned))
            .unwrap_or_default(),
        description: p.help.clone().or_else(|| f.description.clone()),
        violations: Vec::new(),
        locked: None,
        deprecated: f.deprecated,
    }
}

/// Presentation §2's derivation default for a **Repeat group** card title.
fn derive_label_from(items: &Value) -> Option<String> {
    let properties = items.get("properties")?.as_object()?;
    ["name", "id", "title", "host"]
        .into_iter()
        .find(|candidate| properties.contains_key(*candidate))
        .map(str::to_owned)
}

/// The narrower of two values for one keyword, or `None` when narrowness is not defined for it.
fn narrowest(keyword: &str, a: &Value, b: &Value) -> Option<Value> {
    let (x, y) = (a.as_f64()?, b.as_f64()?);
    let pick = match keyword {
        "minimum" | "exclusiveMinimum" | "minLength" | "minItems" | "minProperties" => x.max(y),
        "maximum" | "exclusiveMaximum" | "maxLength" | "maxItems" | "maxProperties" => x.min(y),
        _ => return None,
    };
    serde_json::to_value(pick).ok()
}
