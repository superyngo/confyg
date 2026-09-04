//! Compiler snapshots against a real published Schema (design §11 verification item 1).
//!
//! The outline — one line per node, `path kind widget/occupancy` — is what a human can actually
//! review, and it still fails on any classification, ordering or clamp regression. The full IR
//! for this fixture is tens of thousands of lines, which no reviewer reads.

use confyg_form::affordance::{Density, HostProfile};
use confyg_form::compile::{compile, path_of};
use confyg_form::ir::FormNode;
use confy_core::model::node::Seg;

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
            widget, intended, ..
        } => {
            let clamp = if widget == intended {
                String::new()
            } else {
                format!(" (intended {intended:?})")
            };
            out.push_str(&format!("{indent}{name}: field {widget:?}{clamp}\n"));
        }
        FormNode::Group {
            children, occupancy, ..
        } => {
            out.push_str(&format!("{indent}{name}: group {occupancy:?}\n"));
            for c in children {
                outline(c, out, depth + 1);
            }
        }
        FormNode::Repeat { bounds, .. } => {
            out.push_str(&format!(
                "{indent}{name}: repeat min={:?} max={:?}\n",
                bounds.min, bounds.max
            ));
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
