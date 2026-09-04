//! Design §4 steps 1–6: resolve, merge, classify, order, overlay.
//!
//! `compile` is `project(schema, None, host)`: the all-**Absent** tree. Neither needs a
//! validator, so a Schema that cannot compile for validation still projects a complete form.

use serde_json::{Map, Value};

use crate::affordance::{self, HostProfile};
use crate::constraint;
use crate::facts::{self, AdditionalProperties, SchemaFacts};
use crate::ir::*;
use crate::notice::Notice;
use crate::overlay::DocView;
use crate::vocab::{self, Presentation};

use confy_core::model::any_doc::AnyDocument;
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
    match path_of(node).last() {
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

/// The all-**Absent** tree.
pub fn compile(schema: &Value, host: &HostProfile) -> Compiled {
    project(schema, None, host)
}

/// The whole of design §4: compile the Schema, then overlay the Document if there is one.
pub fn project(schema: &Value, doc: Option<&AnyDocument>, host: &HostProfile) -> Compiled {
    let view = doc.map(|d| DocView::new(d, schema));
    let mut ctx = Ctx {
        root: schema,
        host,
        doc: view.as_ref(),
        notices: Vec::new(),
        visited: Vec::new(),
    };
    let mut root = ctx.build(schema, "#".to_owned(), Vec::new(), None, false);
    let mut notices = ctx.notices;

    // Step 7: sweep unknowns. Only a Document can have keys the Schema does not describe.
    if let Some(view) = view.as_ref() {
        notices.extend(crate::unknown::sweep(&mut root, view));
    }

    Compiled {
        root,
        state: SchemaState {
            projected: true,
            validatable: validatable(schema),
        },
        notices,
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
                keyword: pointer.rsplit('/').next().unwrap_or_default().to_owned(),
                pointer,
                message: e.to_string(),
            })
        }
    }
}

struct Ctx<'a> {
    root: &'a Value,
    host: &'a HostProfile,
    doc: Option<&'a DocView>,
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

    fn follow(
        &mut self,
        reference: &str,
        path: Path,
        key: Option<&str>,
        required: bool,
    ) -> FormNode {
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
                        match narrowed {
                            Some(n) => {
                                out.insert(k.clone(), n);
                            }
                            None => self.notices.push(Notice::new(
                                "form.compile.allof-conflict",
                                format!("`{ptr}` merges conflicting `{k}`; kept the first"),
                            )),
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
            let occupancy = self.occupancy(&path);
            return FormNode::Group {
                meta: self.node_meta(f, p, key, &path),
                path,
                children: children.into_iter().map(|(_, _, n)| n).collect(),
                occupancy,
                toggle: (!required).then_some(GroupToggle {
                    enabled: occupancy != Occupancy::Absent,
                }),
            };
        }

        let is_array = f.ty.as_ref().is_some_and(|t| t.has("array"))
            || (merged.get("items").is_some() && f.ty.is_none());
        if is_array {
            let items_schema = merged
                .get("items")
                .cloned()
                .unwrap_or(Value::Object(Map::new()));
            let item_ptr = format!("{ptr}/items");
            // Step 6 for a collection: one projected item per Document entry.
            let mut items = Vec::new();
            for index in 0..self.doc.map(|d| d.len_at(&path)).unwrap_or(0) {
                let mut item_path = path.clone();
                item_path.push(Seg::Index(index));
                items.push(self.build(&items_schema, item_ptr.clone(), item_path, None, false));
            }
            return FormNode::Repeat {
                meta: self.node_meta(f, p, key, &path),
                occupancy: self.occupancy(&path),
                items,
                bounds: Bounds {
                    min: f.len.min,
                    max: f.len.max,
                },
                label_from: p
                    .label_from
                    .clone()
                    .or_else(|| derive_label_from(&items_schema)),
                item_template: TemplateRef(item_ptr),
                path,
            };
        }

        // An `additionalProperties` / `patternProperties` *schema* with no `properties` is a Map
        // group, which is v0.2.
        if matches!(f.additional, AdditionalProperties::Schema(_))
            || merged.get("patternProperties").is_some()
        {
            self.notices
                .push(excluded("additionalProperties as a schema", "v0.2"));
            return unknown(merged, path);
        }
        if f.ty.as_ref().is_some_and(|t| t.has("object")) {
            let occupancy = self.occupancy(&path);
            return FormNode::Group {
                meta: self.node_meta(f, p, key, &path),
                path,
                children: Vec::new(),
                occupancy,
                toggle: (!required).then_some(GroupToggle {
                    enabled: occupancy != Occupancy::Absent,
                }),
            };
        }

        let raw_fallback = self.doc.is_some_and(|d| d.raw_fallback(&path));
        let (widget, intended, notices) = affordance::resolve(f, p, raw_fallback, self.host);
        self.notices.extend(notices);
        FormNode::Field {
            widget,
            intended,
            presence: self.presence(f, &path),
            meta: FieldMeta {
                node: self.node_meta(f, p, key, &path),
                default: f.default.clone(),
                examples: f.examples.clone(),
                required,
                read_only: f.read_only,
                write_only: f.write_only,
                unit: p.unit.clone(),
                constraints: constraint::extract(f),
                raw: raw_fallback,
            },
            path,
        }
    }

    /// **Presence**: `Absent` with the Schema's default when the key is unwritten, `Invalid` when
    /// the node has its own Violations or its literal is not editable as its declared type, and
    /// `Set` otherwise.
    fn presence(&self, f: &SchemaFacts, path: &Path) -> Presence {
        let Some(doc) = self.doc else {
            return Presence::Absent {
                default: f.default.clone(),
                remarked: None,
            };
        };
        match doc.literal(path) {
            None => Presence::Absent {
                default: f.default.clone(),
                remarked: None,
            },
            Some(literal) => {
                let violations = doc.violations_at(path);
                if violations.is_empty() {
                    Presence::Set { literal }
                } else {
                    Presence::Invalid {
                        literal,
                        violations,
                    }
                }
            }
        }
    }

    fn occupancy(&self, path: &Path) -> Occupancy {
        self.doc
            .map(|d| d.occupancy(path))
            .unwrap_or(Occupancy::Absent)
    }

    fn node_meta(
        &self,
        f: &SchemaFacts,
        p: &Presentation,
        key: Option<&str>,
        path: &Path,
    ) -> NodeMeta {
        NodeMeta {
            title: p
                .label
                .clone()
                .or_else(|| f.title.clone())
                .or_else(|| key.map(str::to_owned))
                .unwrap_or_default(),
            description: p.help.clone().or_else(|| f.description.clone()),
            violations: self.doc.map(|d| d.violations_at(path)).unwrap_or_default(),
            locked: self.doc.and_then(|d| d.locked(path)),
            deprecated: f.deprecated,
        }
    }
}

fn unknown(merged: &Value, path: Path) -> FormNode {
    FormNode::Unknown {
        path,
        raw_preview: merged.to_string(),
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
