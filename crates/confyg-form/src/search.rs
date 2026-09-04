//! **Form search**: fuzzy matching over **Form node** titles, descriptions and **Paths**
//! (presentation §5.3).
//!
//! It lives in the compiler rather than in each host for two reasons the record already
//! settled: a result must be able to move the **Partition** to the section containing the
//! hit, which is a cross-layer interaction; and two host-side implementations guarantee the
//! Web and TUI semantics drift apart.
//!
//! The matcher is `fuzzy-matcher`'s `SkimMatcherV2` — the same crate and version upstream
//! resolves — taken as a direct dependency. Upstream's own search sits in the `session` module
//! ADR 0001 gates off, so nothing here reaches for it.
//!
//! This is not the **Option filter** (§3.1): that one filters choices *inside* a Widget and
//! treats an empty needle as "everything matches". They share no code and no term.

use crate::compile::path_of;
use crate::ir::FormNode;
use confy_core::model::node::{Path, Seg};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

/// One shared matcher: `search` scores every node on every keystroke, so rebuilding
/// `SkimMatcherV2::default()` per call would be pure churn.
static MATCHER: LazyLock<SkimMatcherV2> = LazyLock::new(SkimMatcherV2::default);

/// One result. The `score` crosses the boundary because ordering is the compiler's decision:
/// a host renders the list in the order it arrives, and two hosts cannot re-rank it apart.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Hit {
    pub path: Path,
    /// The node's own title, so a host need not walk the IR again to label the row.
    pub title: String,
    pub score: i64,
}

/// Every node whose title, description or **Path** fuzzy-matches `query`, best first.
///
/// An empty query returns nothing rather than everything: a result list containing the whole
/// tree is not a result. Ties break on **Path**, so the same query never reorders its own
/// results between calls.
pub fn search(root: &FormNode, query: &str) -> Vec<Hit> {
    if query.trim().is_empty() {
        return Vec::new();
    }
    let mut hits = Vec::new();
    collect(root, query, &mut hits);
    hits.sort_by(|a, b| {
        b.score
            .cmp(&a.score)
            .then_with(|| path_text(&a.path).cmp(&path_text(&b.path)))
    });
    hits
}

/// A **Path** rendered for display and for DOM ids: `servers[0].host`. This is the same
/// rendering `web/src/types.ts` `pathText` produces — a divergent one would hand a host a
/// hit it cannot address.
pub fn path_text(path: &Path) -> String {
    let mut out = String::new();
    for seg in path {
        match seg {
            Seg::Key(k) => {
                if !out.is_empty() {
                    out.push('.');
                }
                out.push_str(k);
            }
            Seg::Index(i) => out.push_str(&format!("[{i}]")),
        }
    }
    out
}

fn collect(node: &FormNode, query: &str, out: &mut Vec<Hit>) {
    let (title, description) = prose(node);
    let path = path_of(node);
    if let Some(score) = score(title, description, path, query) {
        out.push(Hit {
            path: path.clone(),
            title: title.to_owned(),
            score,
        });
    }
    match node {
        FormNode::Group { children, .. } => {
            for child in children {
                collect(child, query, out);
            }
        }
        FormNode::Repeat { items, .. } => {
            for item in items {
                collect(item, query, out);
            }
        }
        _ => {}
    }
}

fn prose(node: &FormNode) -> (&str, Option<&str>) {
    match node {
        FormNode::Field { meta, .. } => (&meta.node.title, meta.node.description.as_deref()),
        FormNode::Group { meta, .. } | FormNode::Repeat { meta, .. } => {
            (&meta.title, meta.description.as_deref())
        }
        // An Unknown key and a Cyclic stop carry no Form node prose, but both are real nodes
        // with a Path: a preserved key nobody can find is a preserved key nobody can fix.
        FormNode::Unknown { .. } | FormNode::Cyclic { .. } => ("", None),
    }
}

/// The best of the three axes, so a title hit ranks a node whose Path says nothing and the
/// reverse. An empty axis is skipped rather than scored — every untitled node would otherwise
/// tie on the same empty haystack.
fn score(title: &str, description: Option<&str>, path: &Path, query: &str) -> Option<i64> {
    let rendered = path_text(path);
    [Some(title), description, Some(rendered.as_str())]
        .into_iter()
        .flatten()
        .filter(|axis| !axis.is_empty())
        .filter_map(|axis| MATCHER.fuzzy_match(axis, query))
        .max()
}
