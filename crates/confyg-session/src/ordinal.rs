//! Projection index → `Target.index` conversion (D7), and the Schema slot for a key that is not
//! in the Document yet (D1).
//!
//! Both hazards report success while writing the wrong bytes, which is why they are computed in
//! one place, from the public `Node` tree, and tested in all three **Doc formats** before any
//! `Mutation` exists.
//!
//! Two helpers here duplicate `pub(crate)` upstream logic; `upstream.md` *The upstream bill*
//! item 2 requests them as public API, at which point both bodies can delegate and die.

use confy_core::model::document::ConfigDocument;
use confy_core::model::node::Format;
use confy_core::model::node::{Node, NodeKind, Path};
use confy_core::model::{any_doc::AnyDocument, document::DocFormat};

/// Turn an index in **projection** space — entries only, comments excluded, which is the space
/// the Form IR counts in — into the child ordinal a `Target` wants.
///
/// A `Target.index` counts *every* child of the parent node, comments included: inserting "after
/// the first entry" of a file that opens with three comment lines is ordinal 4, not 1. Getting
/// this wrong misplaces text without failing anything (D7).
///
/// An index past the last entry means "append", and yields the parent's child count.
// PORTED: `confy-core` `session::insertion` + `model::yaml::edit::block::root_prefix_offset`.
pub fn child_ordinal(doc: &AnyDocument, parent: &Path, projection_index: usize) -> usize {
    let tree = doc.project();
    let Some(node) = node_at(&tree.root, parent) else {
        return projection_index;
    };
    raw_ordinal(node, projection_index).saturating_sub(root_prefix_offset(doc, node, parent))
}

/// The same conversion in the parent's own child-index space, before YAML's root prefix is
/// subtracted — the space the comment back-off and the TOML partition both reason in.
fn raw_ordinal(node: &Node, projection_index: usize) -> usize {
    node.children
        .iter()
        .enumerate()
        .filter(|(_, c)| !is_comment(c))
        .map(|(i, _)| i)
        .nth(projection_index)
        .unwrap_or(node.children.len())
}

/// Where a key the Document does not hold yet belongs: its **Schema `properties` order** position
/// among the siblings that *are* present, converted to a child ordinal — never appended blindly
/// (design §8).
///
/// TOML's root does not clamp, so legality wins over Schema order there: a plain key may not land
/// after a `[table]`/`[[aot]]` header, which would silently re-key it into that section (D1).
pub fn schema_slot(doc: &AnyDocument, parent: &Path, key: &str, schema_order: &[String]) -> usize {
    let tree = doc.project();
    let Some(node) = node_at(&tree.root, parent) else {
        return 0;
    };
    let rank = |k: &str| {
        schema_order
            .iter()
            .position(|s| s == k)
            .unwrap_or(usize::MAX)
    };
    let target_rank = rank(key);
    let before = node
        .children
        .iter()
        .filter(|c| !is_comment(c))
        .filter(|c| rank(&c.key) <= target_rank)
        .count();

    // A leading comment block documents the entry *below* it, so a new key inserted "before
    // `ca`" belongs above `ca`'s comment, not between the comment and the key it describes.
    // At the end of the parent there is no such entry, so a trailing block is left alone.
    let mut ordinal = raw_ordinal(node, before);
    while ordinal > 0 && ordinal < node.children.len() && is_comment(&node.children[ordinal - 1]) {
        ordinal -= 1;
    }
    let ordinal = clamp_to_partition(doc, node, ordinal);
    ordinal.saturating_sub(root_prefix_offset(doc, node, parent))
}

/// A plain key may only land at or before the parent's first capturing header. A `[a.b]` dotted
/// table opens no scope, so it is not a boundary.
// PORTED: `confy-core` `model::cst_edit::move_paste::check_partition` (leaf-like half).
fn clamp_to_partition(doc: &AnyDocument, parent: &Node, ordinal: usize) -> usize {
    if doc.format() != DocFormat::Toml {
        return ordinal;
    }
    let split = parent
        .children
        .iter()
        .position(|c| {
            matches!(c.kind, NodeKind::Table | NodeKind::ArrayOfTables)
                && c.format != Format::Dotted
        })
        .unwrap_or(parent.children.len());
    ordinal.min(split)
}

/// YAML root indices are *container* indices: the leading root-level comment blocks precede the
/// mapping node itself rather than living inside it, so they are not addressable slots.
fn root_prefix_offset(doc: &AnyDocument, root: &Node, parent: &Path) -> usize {
    if !parent.is_empty() || doc.format() != DocFormat::Yaml {
        return 0;
    }
    root.children.iter().take_while(|c| is_comment(c)).count()
}

fn is_comment(node: &Node) -> bool {
    matches!(node.kind, NodeKind::Comment(_))
}

fn node_at<'a>(root: &'a Node, path: &Path) -> Option<&'a Node> {
    let mut current = root;
    for seg in path {
        current = match seg {
            confy_core::model::node::Seg::Key(k) => {
                current.children.iter().find(|c| &c.key == k)?
            }
            confy_core::model::node::Seg::Index(i) => {
                current.children.iter().filter(|c| !is_comment(c)).nth(*i)?
            }
        };
    }
    Some(current)
}
