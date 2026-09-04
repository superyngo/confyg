//! Design §4 step 7: the unknown sweep, and the violation summary hosts render (C6).
//!
//! An **Unknown key** carries a **Notice**, not a **Violation**, unless the validator genuinely
//! reported one: under `additionalProperties: true` an extra key breaks no rule and confyg does
//! not fabricate failures (design §7 B18). Under `additionalProperties: false` the Violation the
//! validator did report is already attached to the container by the overlay, and the Notice here
//! is what names the key — the validator only names it in the message.
//!
//! One `FormNode::Unknown` per unknown key, appended after its parent's known children: the IR's
//! `Unknown` carries a **Path**, so a node per key is what lets a host address, preview and
//! remove one. Design §4 step 7's "one Unknown group per parent" is that trailing run of nodes.

use confy_core::model::node::{Path, Seg};
use serde::{Deserialize, Serialize};

use crate::compile::{key_of, path_of};
use crate::ir::{FormNode, Presence, SchemaCompileError, SchemaState};
use crate::notice::Notice;
use crate::overlay::DocView;

/// Whether the document could be validated at all, and if not, why. Never "no problems" when the
/// answer is "unknown" (C6, D8).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Validation {
    Available,
    Unavailable { keyword: String, pointer: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SummaryItem {
    pub path: Path,
    pub title: String,
    pub keyword: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    pub items: Vec<SummaryItem>,
    pub validation: Validation,
}

/// Attach every Document key with no **Form node** to its parent, in one pass per container.
pub fn sweep(node: &mut FormNode, doc: &DocView) -> Vec<Notice> {
    let mut notices = Vec::new();
    sweep_into(node, doc, &mut notices);
    notices
}

fn sweep_into(node: &mut FormNode, doc: &DocView, notices: &mut Vec<Notice>) {
    match node {
        FormNode::Group { path, children, .. } => {
            let known: Vec<String> = children
                .iter()
                .filter_map(|c| key_of(c).map(str::to_owned))
                .collect();
            for key in doc.extra_keys(path, &known) {
                let mut child_path = path.clone();
                child_path.push(Seg::Key(key.clone()));
                notices.push(Notice::new(
                    "form.unknown.key",
                    format!("`{key}` is not described by the Schema; it is preserved as authored"),
                ));
                let raw_preview = doc.literal(&child_path).unwrap_or_default();
                children.push(FormNode::Unknown {
                    path: child_path,
                    raw_preview,
                });
            }
            for child in children.iter_mut() {
                sweep_into(child, doc, notices);
            }
        }
        FormNode::Repeat { items, .. } => {
            for item in items.iter_mut() {
                sweep_into(item, doc, notices);
            }
        }
        _ => {}
    }
}

/// The violation summary: one item per attributed **Violation**, depth first, plus the
/// document-level validation state.
pub fn summary(root: &FormNode, state: &SchemaState) -> Summary {
    let mut items = Vec::new();
    collect(root, &mut items);
    Summary {
        items,
        validation: match &state.validatable {
            Ok(()) => Validation::Available,
            Err(SchemaCompileError {
                keyword, pointer, ..
            }) => Validation::Unavailable {
                keyword: keyword.clone(),
                pointer: pointer.clone(),
            },
        },
    }
}

fn collect(node: &FormNode, items: &mut Vec<SummaryItem>) {
    let (violations, title) = match node {
        FormNode::Field { meta, .. } => (&meta.node.violations, &meta.node.title),
        FormNode::Group { meta, .. } | FormNode::Repeat { meta, .. } => {
            (&meta.violations, &meta.title)
        }
        _ => (&EMPTY, &EMPTY_TITLE),
    };
    for v in violations.iter() {
        items.push(SummaryItem {
            path: path_of(node).clone(),
            title: title.to_string(),
            keyword: v.keyword.clone(),
            message: v.message.clone(),
        });
    }
    // A Field's `Invalid` violations are the same list its meta already carries, so they are not
    // collected twice; this arm only asserts that assumption stays true.
    if let FormNode::Field {
        presence: Presence::Invalid { violations, .. },
        meta,
        ..
    } = node
    {
        debug_assert_eq!(violations.len(), meta.node.violations.len());
    }
    match node {
        FormNode::Group { children, .. } => children.iter().for_each(|c| collect(c, items)),
        FormNode::Repeat { items: entries, .. } => entries.iter().for_each(|c| collect(c, items)),
        _ => {}
    }
}

static EMPTY: Vec<confy_core::schema::types::Violation> = Vec::new();
static EMPTY_TITLE: String = String::new();
