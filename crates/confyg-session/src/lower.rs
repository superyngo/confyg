//! Design §5: lower a **Setter intent** onto `confy-core` `Mutation`s.
//!
//! Two rules shape every arm:
//!
//! - **Minimal write.** A value equal to the effective `default` lowers to `Delete`, never to a
//!   `Replace` that writes the default out (ADR 0003).
//! - **Soft constraint.** A value that violates the Schema is written and warned about, never
//!   refused. `Refused` is only ever returned for an intent the host should not have offered —
//!   a bug signal, not a user-facing path.

use confy_core::model::any_doc::AnyDocument;
use confy_core::model::document::{ConfigDocument, Mutation, OnCollision, Target};
use confy_core::model::node::{Node, NodeKind, Path, Seg};
use confyg_form::ir::{FormNode, Presence};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::ordinal::schema_slot;

/// Everything a host can ask the session to write. Extended, never renamed, by later tasks.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum SetterIntent {
    SetValue { path: Path, value: Value },
    Unset { path: Path },
}

/// An intent the host should not have offered. Carries the reason so the bug is nameable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Refused {
    pub reason: String,
}

impl Refused {
    fn new(reason: impl Into<String>) -> Self {
        Refused {
            reason: reason.into(),
        }
    }
}

pub fn lower(
    intent: &SetterIntent,
    ir: &FormNode,
    doc: &AnyDocument,
) -> Result<Vec<Mutation>, Refused> {
    match intent {
        SetterIntent::SetValue { path, value } => set_value(path, value, ir, doc),
        SetterIntent::Unset { path } => unset(path, ir, doc),
    }
}

fn set_value(
    path: &Path,
    value: &Value,
    ir: &FormNode,
    doc: &AnyDocument,
) -> Result<Vec<Mutation>, Refused> {
    let field = find(ir, path).ok_or_else(|| Refused::new("no Field at that path"))?;
    let (presence, meta) = match field {
        FormNode::Field { presence, meta, .. } => (presence, meta),
        _ => return Err(Refused::new("that path is not a Field")),
    };
    if meta.read_only {
        return Err(Refused::new("a readOnly Field has no write affordance"));
    }
    if meta.node.locked.is_some() {
        return Err(Refused::new("a locked Field has no write affordance"));
    }

    // Minimal write: the effective default is written by *absence*, so writing it means removing
    // the key — and an already-absent key needs no mutation at all.
    if meta.default.as_ref() == Some(value) {
        return Ok(match presence {
            Presence::Absent { .. } => Vec::new(),
            _ if meta.required => vec![replace(doc, path, value)?],
            _ => vec![Mutation::Delete { path: path.clone() }],
        });
    }

    match presence {
        Presence::Set { .. } | Presence::Invalid { .. } => {
            Ok(replace_with_comment(doc, path, value)?)
        }
        Presence::Absent { .. } => Ok(vec![insert(doc, ir, path, value)?]),
    }
}

fn unset(path: &Path, ir: &FormNode, doc: &AnyDocument) -> Result<Vec<Mutation>, Refused> {
    let field = find(ir, path).ok_or_else(|| Refused::new("no Field at that path"))?;
    let (presence, meta) = match field {
        FormNode::Field { presence, meta, .. } => (presence, meta),
        _ => return Err(Refused::new("that path is not a Field")),
    };
    if meta.required {
        return Err(Refused::new("a required Field has no Unset affordance"));
    }
    if meta.node.locked.is_some() {
        return Err(Refused::new("a locked Field has no write affordance"));
    }
    let _ = doc;
    match presence {
        Presence::Absent { .. } => Ok(Vec::new()),
        _ => Ok(vec![Mutation::Delete { path: path.clone() }]),
    }
}

/// `Replace`, plus the trailing-comment restoration YAML needs: its `Replace` swaps the whole
/// `key: value` entry and drops the comment, which Write-neutrality does not allow.
fn replace_with_comment(
    doc: &AnyDocument,
    path: &Path,
    value: &Value,
) -> Result<Vec<Mutation>, Refused> {
    let mut out = vec![replace(doc, path, value)?];
    if !doc.replace_preserves_trailing_comment() {
        if let Some(text) =
            node_at(&doc.project().root, path).and_then(|n| n.trailing_comment.clone())
        {
            out.push(Mutation::SetTrailingComment {
                path: path.clone(),
                comment: Some(text),
            });
        }
    }
    Ok(out)
}

fn replace(doc: &AnyDocument, path: &Path, value: &Value) -> Result<Mutation, Refused> {
    let key = leaf_key(path);
    Ok(Mutation::Replace {
        path: path.clone(),
        fragment: doc.scalar_fragment(key.as_deref(), &repr(value)),
    })
}

/// An absent key is inserted at its **Schema `properties` order** position among the siblings
/// that are present, converted from projection space into a child ordinal (D1, D7).
fn insert(
    doc: &AnyDocument,
    ir: &FormNode,
    path: &Path,
    value: &Value,
) -> Result<Mutation, Refused> {
    let key = leaf_key(path).ok_or_else(|| Refused::new("cannot insert a keyless node by key"))?;
    let mut parent = path.clone();
    parent.pop();

    let order = sibling_order(ir, &parent);
    let index = if doc_has(doc, &parent) {
        schema_slot(doc, &parent, &key, &order)
    } else {
        // The parent is absent too; Task 11 owns absent-parent lowering.
        return Err(Refused::new("the parent collection is absent"));
    };

    Ok(Mutation::Insert {
        target: Target { parent, index },
        fragment: doc.scalar_fragment(Some(&key), &repr(value)),
        on_collision: OnCollision::Cancel,
        suggested_key: Some(key),
    })
}

/// The Schema's property order for `parent`, read off the compiled IR — the compiler already
/// ordered the children (design §4 step 5), so the order is not re-derived from the Schema.
fn sibling_order(ir: &FormNode, parent: &Path) -> Vec<String> {
    let Some(FormNode::Group { children, .. }) = find(ir, parent) else {
        return Vec::new();
    };
    children
        .iter()
        .filter(|c| !matches!(c, FormNode::Unknown { .. }))
        .filter_map(|c| match confyg_form::compile::key_of(c) {
            Some(k) => Some(k.to_owned()),
            None => None,
        })
        .collect()
}

/// A value literal every format accepts: JSON's own notation is valid TOML and valid YAML
/// (flow) for every scalar confyg writes.
fn repr(value: &Value) -> String {
    value.to_string()
}

fn leaf_key(path: &Path) -> Option<String> {
    match path.last() {
        Some(Seg::Key(k)) => Some(k.clone()),
        _ => None,
    }
}

fn doc_has(doc: &AnyDocument, path: &Path) -> bool {
    node_at(&doc.project().root, path).is_some()
}

fn find<'a>(ir: &'a FormNode, path: &Path) -> Option<&'a FormNode> {
    if confyg_form::compile::path_of(ir) == path {
        return Some(ir);
    }
    let children: &[FormNode] = match ir {
        FormNode::Group { children, .. } => children,
        FormNode::Repeat { items, .. } => items,
        _ => return None,
    };
    children.iter().find_map(|c| find(c, path))
}

fn node_at<'a>(root: &'a Node, path: &Path) -> Option<&'a Node> {
    let mut current = root;
    for seg in path {
        current = match seg {
            Seg::Key(k) => current.children.iter().find(|c| &c.key == k)?,
            Seg::Index(i) => current
                .children
                .iter()
                .filter(|c| !matches!(c.kind, NodeKind::Comment(_)))
                .nth(*i)?,
        };
    }
    Some(current)
}
