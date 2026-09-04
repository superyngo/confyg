//! Template fragments: the text a `Mutation::Insert` carries.
//!
//! `Insert` *adapts* raw text rather than rejecting it, so a wrong shape writes a structurally
//! different document and reports success (`upstream.md` *Fragment contract*). Every fragment
//! confyg writes is therefore built here, from the Schema, in one place.
//!
//! **Emission style** is the D2 asymmetry: TOML expresses "first item of an absent collection"
//! versus "next item of an existing one" as a *`Target` choice*, not as fragment text. A
//! header-bearing fragment goes into the parent; a headerless one is addressed at the
//! collection's own **Path** and the engine synthesizes the header (design §8).

use confy_core::model::document::DocFormat;
use confyg_form::ir::TemplateRef;
use serde_json::{Map, Value};

/// Which end of the D2 asymmetry a fragment is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Emission {
    /// Addressed at the collection's own Path: no header, the engine synthesizes it.
    Headerless,
    /// Addressed at the parent: carries its own key or `[[header]]`.
    HeaderBearing,
}

/// Render one template item for the subschema at `ptr`.
///
/// The key a header-bearing fragment needs is read off the pointer (`#/properties/servers/items`
/// → `servers`), which is what makes `TemplateRef` sufficient: the IR stores a pointer rather
/// than inlined text precisely so fragments are rendered on demand.
pub fn fragment(schema: &Value, ptr: &TemplateRef, fmt: DocFormat, style: Emission) -> String {
    let sub = resolve(schema, &ptr.0).cloned().unwrap_or(Value::Null);
    let key = owner_key(&ptr.0);
    let is_item = ptr.0.ends_with("/items");
    let members = placeholder_members(&sub);

    match fmt {
        DocFormat::Toml => toml_fragment(&members, &key, is_item, style),
        DocFormat::Json => json_fragment(&members, &key, is_item, style),
        DocFormat::Yaml => yaml_fragment(&members, &key, is_item, style),
    }
}

/// **Minimal write** for generated text: a member with an effective `default` is written by
/// absence, so a template never emits it. Everything else gets a typed placeholder.
fn placeholder_members(sub: &Value) -> Vec<(String, String)> {
    let Some(props) = sub.get("properties").and_then(Value::as_object) else {
        // A scalar collection: one keyless placeholder.
        return vec![(String::new(), placeholder(sub))];
    };
    props
        .iter()
        .filter(|(_, s)| s.get("default").is_none())
        .map(|(k, s)| (k.clone(), placeholder(s)))
        .collect()
}

/// The empty value of a member's type — JSON notation, which every backend parses.
fn placeholder(sub: &Value) -> String {
    match sub.get("type").and_then(Value::as_str) {
        Some("integer") | Some("number") => "0".into(),
        Some("boolean") => "false".into(),
        Some("array") => "[]".into(),
        Some("object") => "{}".into(),
        _ => "\"\"".into(),
    }
}

fn toml_fragment(
    members: &[(String, String)],
    key: &str,
    is_item: bool,
    style: Emission,
) -> String {
    let body: String = members
        .iter()
        .map(|(k, v)| {
            if k.is_empty() {
                format!("{v}\n")
            } else {
                format!("{k} = {v}\n")
            }
        })
        .collect();
    match style {
        Emission::Headerless => body,
        Emission::HeaderBearing if is_item => format!("[[{key}]]\n{body}"),
        Emission::HeaderBearing => format!("[{key}]\n{body}"),
    }
}

fn json_fragment(
    members: &[(String, String)],
    key: &str,
    is_item: bool,
    style: Emission,
) -> String {
    let inner: String = members
        .iter()
        .map(|(k, v)| format!("\"{k}\": {v}"))
        .collect::<Vec<_>>()
        .join(", ");
    let object = if members.first().is_some_and(|(k, _)| k.is_empty()) {
        members[0].1.clone()
    } else {
        format!("{{ {inner} }}")
    };
    match style {
        Emission::Headerless => object,
        Emission::HeaderBearing if is_item => format!("\"{key}\": [{object}]\n"),
        Emission::HeaderBearing => format!("\"{key}\": {object}\n"),
    }
}

fn yaml_fragment(
    members: &[(String, String)],
    key: &str,
    is_item: bool,
    style: Emission,
) -> String {
    let lines: Vec<String> = members
        .iter()
        .map(|(k, v)| {
            if k.is_empty() {
                v.clone()
            } else {
                format!("{k}: {v}")
            }
        })
        .collect();
    match style {
        Emission::Headerless if is_item => format!("- {}\n", lines.join("\n  ")),
        Emission::Headerless => format!("{}\n", lines.join("\n")),
        Emission::HeaderBearing if is_item => {
            format!("{key}:\n  - {}\n", lines.join("\n    "))
        }
        Emission::HeaderBearing => {
            let indented: Vec<String> = lines.iter().map(|l| format!("  {l}")).collect();
            format!("{key}:\n{}\n", indented.join("\n"))
        }
    }
}

/// The property name that owns this pointer: the last `properties/<name>` segment.
fn owner_key(ptr: &str) -> String {
    let segs: Vec<&str> = ptr.trim_start_matches("#/").split('/').collect();
    segs.iter()
        .rev()
        .zip(segs.iter().rev().skip(1))
        .find(|(_, prev)| **prev == "properties")
        .map(|(name, _)| (*name).to_owned())
        .unwrap_or_default()
}

/// Follow a JSON pointer, `$ref` hops included, non-panicking.
fn resolve<'a>(root: &'a Value, ptr: &str) -> Option<&'a Value> {
    let mut current = root;
    for raw in ptr
        .trim_start_matches('#')
        .split('/')
        .filter(|s| !s.is_empty())
    {
        let seg = raw.replace("~1", "/").replace("~0", "~");
        current = match current {
            Value::Object(map) => lookup(map, &seg)?,
            Value::Array(items) => items.get(seg.parse::<usize>().ok()?)?,
            _ => return None,
        };
        if let Some(next) = current.get("$ref").and_then(Value::as_str) {
            current = resolve(root, next)?;
        }
    }
    Some(current)
}

fn lookup<'a>(map: &'a Map<String, Value>, seg: &str) -> Option<&'a Value> {
    map.get(seg)
}
