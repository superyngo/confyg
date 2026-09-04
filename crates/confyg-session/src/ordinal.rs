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
    slot(doc, parent, key, schema_order, Shape::PlainKey)
}

/// The same slot for a **header-bearing** fragment — a `[table]`, `[[aot]]` or their JSON/YAML
/// equivalents. In TOML the clamp runs the *other* way: a header may not land *before* a plain
/// key, because it would capture it into the new section, which is why the engine refuses the
/// Insert outright. Legality wins over Schema order here for the same reason it does above.
pub fn header_slot(doc: &AnyDocument, parent: &Path, key: &str, schema_order: &[String]) -> usize {
    slot(doc, parent, key, schema_order, Shape::Header)
}

/// Which way the TOML partition clamps. The two slots differ in nothing else.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Shape {
    PlainKey,
    Header,
}

fn slot(
    doc: &AnyDocument,
    parent: &Path,
    key: &str,
    schema_order: &[String],
    shape: Shape,
) -> usize {
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
    let ordinal = match shape {
        Shape::PlainKey => clamp_to_partition(doc, node, ordinal),
        Shape::Header => clamp_past_headers(doc, node, ordinal),
    };
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
        .position(|c| is_capturing_header(c))
        .unwrap_or(parent.children.len());
    ordinal.min(split)
}

/// The floor for a header. Upstream's rule is coarser than "captures nothing": a header-like
/// fragment is legal only at an index `>= split`, the parent's *first* capturing-header child
/// (`check_partition`), so a section can never precede an existing one's position.
///
/// Two hard rules can collide at that floor: landing there may split a comment block from the
/// entry below it (D1), which is the misattribution the back-off above exists to prevent. When
/// they do, **Schema order** is the one that yields — it is a preference, the other two are not.
// PORTED: `confy-core` `model::cst_edit::move_paste::check_partition` (header-like half).
fn clamp_past_headers(doc: &AnyDocument, parent: &Node, ordinal: usize) -> usize {
    if doc.format() != DocFormat::Toml {
        return ordinal;
    }
    let split = parent
        .children
        .iter()
        .position(|c| is_capturing_header(c))
        .unwrap_or(parent.children.len());
    let mut index = ordinal.max(split);
    if index > 0 && index < parent.children.len() && is_comment(&parent.children[index - 1]) {
        // Step over the entry that comment documents, rather than between the two.
        index += 1;
    }
    index
}

/// A `[table]`/`[[aot]]` header that opens a scope: everything after it belongs to that section
/// until the next header. A dotted-key table is not one.
fn is_capturing_header(node: &Node) -> bool {
    matches!(node.kind, NodeKind::Table | NodeKind::ArrayOfTables) && node.format != Format::Dotted
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
