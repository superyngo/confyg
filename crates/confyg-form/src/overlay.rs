//! Design §4 step 6: overlay the **Document** onto the tree — **Presence**, **Occupancy**,
//! `locked`, and **Violation** attribution.
//!
//! The two attribution rules in design §4 step 6 are consequences of `PointerMap::resolve`
//! walking *up* the pointer, so they are asserted here rather than re-implemented: a Violation
//! whose pointer resolves to an ancestor lands on that ancestor, and an unresolvable pointer
//! lands on the root.

use std::collections::HashMap;

use serde_json::Value;

use confy_core::model::any_doc::AnyDocument;
use confy_core::model::document::ConfigDocument;
use confy_core::model::node::{Node, NodeKind, Path, Seg};
use confy_core::schema::types::Violation;
use confy_core::schema::validate::validate;
use confy_core::schema::value_bridge::bridge;

use crate::ir::{Locked, LockedReason, Occupancy};

/// Everything the overlay needs from one Document, computed once per projection.
pub struct DocView {
    root: Node,
    violations: HashMap<Path, Vec<Violation>>,
}

impl DocView {
    /// Project the Document and validate it against `schema`. A Schema that cannot compile for
    /// validation yields no Violations and still overlays every literal (D8).
    pub fn new(doc: &AnyDocument, schema: &Value) -> DocView {
        let root = doc.project().root;
        let mut violations: HashMap<Path, Vec<Violation>> = HashMap::new();

        if let Ok((value, _warnings)) = doc.to_value() {
            let (json, map) = bridge(&root, &value);
            if let Ok(validator) = jsonschema::Validator::new(schema) {
                for v in validate(&json, &validator, &map) {
                    violations.entry(v.path.clone()).or_default().push(v);
                }
            }
        }

        DocView { root, violations }
    }

    pub fn node(&self, path: &Path) -> Option<&Node> {
        let mut current = &self.root;
        for seg in path {
            current = match seg {
                Seg::Key(k) => current.children.iter().find(|c| &c.key == k)?,
                Seg::Index(i) => entries(current).nth(*i)?,
            };
        }
        Some(current)
    }

    pub fn violations_at(&self, path: &Path) -> Vec<Violation> {
        self.violations.get(path).cloned().unwrap_or_default()
    }

    /// The literal exactly as authored, for a scalar leaf.
    pub fn literal(&self, path: &Path) -> Option<String> {
        let node = self.node(path)?;
        node.value.clone().or_else(|| Some(String::new()))
    }

    pub fn occupancy(&self, path: &Path) -> Occupancy {
        match self.node(path) {
            None => Occupancy::Absent,
            Some(node) if entries(node).next().is_none() => Occupancy::Empty,
            Some(_) => Occupancy::Populated,
        }
    }

    /// How many real entries a container holds, comments excluded.
    pub fn len_at(&self, path: &Path) -> usize {
        self.node(path).map(|n| entries(n).count()).unwrap_or(0)
    }

    /// A YAML alias, anchor, or merge-key node is `read_only` in the projection: confyg renders
    /// its resolved value and offers no write affordance (B21).
    pub fn locked(&self, path: &Path) -> Option<Locked> {
        self.node(path)?.read_only.then_some(Locked {
            reason: LockedReason::YamlAlias,
        })
    }

    /// Whether this node's own violations say the literal cannot be edited as its declared type,
    /// which is what activates **Raw literal fallback** (A23). The validator's own verdict is
    /// used, so a form warning can never disagree with a Violation.
    pub fn raw_fallback(&self, path: &Path) -> bool {
        self.violations
            .get(path)
            .is_some_and(|vs| vs.iter().any(|v| v.keyword == "type"))
    }

    /// Document keys under `parent` with no corresponding Form node. Task 8 sweeps these.
    pub fn extra_keys(&self, parent: &Path, known: &[String]) -> Vec<String> {
        let Some(node) = self.node(parent) else {
            return Vec::new();
        };
        entries(node)
            .filter(|c| !c.key.is_empty() && !known.iter().any(|k| k == &c.key))
            .map(|c| c.key.clone())
            .collect()
    }
}

/// A container's real children — `Comment` nodes are projection artefacts, not entries, and the
/// **projection** index space excludes them.
pub fn entries(node: &Node) -> impl Iterator<Item = &Node> {
    node.children
        .iter()
        .filter(|c| !matches!(c.kind, NodeKind::Comment(_)))
}
