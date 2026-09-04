//! Compiler snapshots against a real published Schema (design §11 verification item 1).
//!
//! The outline — one line per node, `path kind widget/occupancy` — is what a human can actually
//! review, and it still fails on any classification, ordering or clamp regression. The full IR
//! for this fixture is tens of thousands of lines, which no reviewer reads.

use confy_core::model::any_doc::AnyDocument;
use confy_core::model::document::DocFormat;
use confy_core::model::node::Seg;
use confyg_form::affordance::{Density, HostProfile};
use confyg_form::compile::{compile, path_of, project};
use confyg_form::ir::{FormNode, Presence};

fn desktop() -> HostProfile {
    HostProfile {
        can_mask: true,
        can_slide: true,
        can_filter_options: false,
        density: Density::Desktop,
    }
}

fn tui() -> HostProfile {
    HostProfile {
        can_mask: false,
        can_slide: false,
        can_filter_options: false,
        density: Density::Desktop,
    }
}

fn outline(node: &FormNode, out: &mut String, depth: usize) {
    let path: Vec<String> = path_of(node)
        .iter()
        .map(|s| match s {
            Seg::Key(k) => k.clone(),
            Seg::Index(i) => i.to_string(),
        })
        .collect();
    let name = if path.is_empty() {
        "<root>".to_owned()
    } else {
        path.join(".")
    };
    let indent = "  ".repeat(depth);
    match node {
        FormNode::Field {
            widget,
            intended,
            presence,
            ..
        } => {
            let clamp = if widget == intended {
                String::new()
            } else {
                format!(" (intended {intended:?})")
            };
            // The presence *kind* only: literals differ per format by design, the shape must not.
            let state = match presence {
                Presence::Absent { .. } => "absent",
                Presence::Set { .. } => "set",
                Presence::Invalid { .. } => "invalid",
            };
            out.push_str(&format!(
                "{indent}{name}: field {widget:?}{clamp} {state}\n"
            ));
        }
        FormNode::Group {
            children,
            occupancy,
            ..
        } => {
            out.push_str(&format!("{indent}{name}: group {occupancy:?}\n"));
            for c in children {
                outline(c, out, depth + 1);
            }
        }
        FormNode::Repeat {
            bounds,
            items,
            occupancy,
            ..
        } => {
            out.push_str(&format!(
                "{indent}{name}: repeat {occupancy:?} min={:?} max={:?}\n",
                bounds.min, bounds.max
            ));
            for i in items {
                outline(i, out, depth + 1);
            }
        }
        FormNode::Unknown { .. } => out.push_str(&format!("{indent}{name}: unknown\n")),
        FormNode::Cyclic { schema_ptr, .. } => {
            out.push_str(&format!("{indent}{name}: cyclic {schema_ptr}\n"))
        }
    }
}

fn eslintrc() -> serde_json::Value {
    serde_json::from_str(include_str!("fixtures/eslintrc.json")).unwrap()
}

#[test]
fn eslintrc_outline_on_a_desktop_host() {
    let c = compile(&eslintrc(), &desktop());
    let mut out = String::new();
    outline(&c.root, &mut out, 0);
    insta::assert_snapshot!(out);
}

#[test]
fn eslintrc_outline_on_a_host_that_cannot_mask_or_slide() {
    let c = compile(&eslintrc(), &tui());
    let mut out = String::new();
    outline(&c.root, &mut out, 0);
    insta::assert_snapshot!(out);
}

/// Verification item 1: the same logical document in all three Doc formats must produce the
/// same IR modulo literals. One snapshot, asserted three times.
#[test]
fn one_document_three_formats_one_outline() {
    let schema = serde_json::json!({"properties":{
        "host":{"type":"string"},
        "port":{"type":"integer","minimum":1,"maximum":65535},
        "servers":{"type":"array","items":{"type":"object",
            "properties":{"name":{"type":"string"}}}},
        "tls":{"type":"object","properties":{"on":{"type":"boolean"}}}}});
    let sources = [
        (
            DocFormat::Toml,
            "host = \"a\"\nport = 80\n\n[tls]\non = true\n\n[[servers]]\nname = \"web-1\"\n",
        ),
        (
            DocFormat::Json,
            "{\"host\":\"a\",\"port\":80,\"tls\":{\"on\":true},\"servers\":[{\"name\":\"web-1\"}]}",
        ),
        (
            DocFormat::Yaml,
            "host: a\nport: 80\ntls:\n  on: true\nservers:\n  - name: web-1\n",
        ),
    ];
    for (fmt, src) in sources {
        let doc = AnyDocument::from_str_as(src, fmt).expect("parse");
        let c = project(&schema, Some(&doc), &desktop());
        let mut out = String::new();
        outline(&c.root, &mut out, 0);
        insta::assert_snapshot!("one_document_three_formats", out, &format!("{fmt:?}"));
    }
}
