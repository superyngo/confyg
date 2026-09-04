//! Template generation: the one place confyg *authors* text.
//!
//! Comments are emitted here and nowhere else — an existing **Document**'s comments are never
//! touched. **Comment policy** is derived from the file, never chosen by the user (D4): a strict
//! `.json` gets no comments at all, and the Schema hint moves into the document body instead.
//!
//! The acceptance test for the hint is not its spelling but C3: the file confyg wrote must
//! resolve its own Schema when it is reopened, which is `detect_hint`'s job.

use confy_core::model::document::DocFormat;
use confyg_form::ir::{FormNode, Presence};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// How much of the Schema a Template writes out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TemplateStrategy {
    RequiredOnly,
    WithDefaults,
    Everything,
}

/// Whether this file may carry comments. Derived, never a setting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CommentPolicy {
    Allow,
    Deny,
}

/// TOML and YAML always allow comments. JSON allows them only when the file says it tolerates
/// them: a `.jsonc` extension, or a `.json` that arrived already carrying comments — which is
/// evidence about its consumer, not about the format.
pub fn comment_policy(path: Option<&str>, fmt: DocFormat, had_comments: bool) -> CommentPolicy {
    if fmt != DocFormat::Json {
        return CommentPolicy::Allow;
    }
    let jsonc = path.is_some_and(|p| p.to_ascii_lowercase().ends_with(".jsonc"));
    if jsonc || had_comments {
        CommentPolicy::Allow
    } else {
        // A new file with no extension denies until the format dialog sets one: denying is the
        // recoverable mistake, since a comment written into strict JSON breaks every parser.
        CommentPolicy::Deny
    }
}

/// Write a Template for `ir`. Only Fields the strategy selects are emitted, and a Field's
/// `title`/`description` become leading comments when the policy allows them.
pub fn generate(
    schema: &Value,
    ir: &FormNode,
    target: DocFormat,
    strategy: TemplateStrategy,
    comments: CommentPolicy,
) -> String {
    let id = schema.get("$id").and_then(Value::as_str);
    let mut out = String::new();

    // JSON with no comment budget carries the hint as a real member instead.
    let inline_hint = matches!(target, DocFormat::Json) && comments == CommentPolicy::Deny;
    if let Some(id) = id {
        if !inline_hint {
            out.push_str(&hint_line(id, target));
            out.push('\n');
        }
    }

    let mut body = Vec::new();
    if inline_hint {
        if let Some(id) = id {
            body.push(format!("  \"$schema\": {}", Value::from(id)));
        }
    }
    emit(ir, strategy, comments, target, &mut body);

    match target {
        DocFormat::Json => {
            out.push_str("{\n");
            out.push_str(&body.join(",\n"));
            out.push_str("\n}\n");
        }
        _ => {
            for line in body {
                out.push_str(&line);
                out.push('\n');
            }
        }
    }
    out
}

/// Each format's own hint convention.
fn hint_line(id: &str, fmt: DocFormat) -> String {
    match fmt {
        DocFormat::Toml => format!("#:schema {id}\n"),
        DocFormat::Yaml => format!("# yaml-language-server: $schema={id}\n"),
        DocFormat::Json => format!("// $schema={id}\n"),
    }
}

fn emit(
    node: &FormNode,
    strategy: TemplateStrategy,
    comments: CommentPolicy,
    fmt: DocFormat,
    out: &mut Vec<String>,
) {
    match node {
        FormNode::Group { children, .. } => {
            for child in children {
                emit(child, strategy, comments, fmt, out);
            }
        }
        FormNode::Field {
            path,
            presence,
            meta,
            ..
        } => {
            let selected = match strategy {
                TemplateStrategy::RequiredOnly => meta.required,
                TemplateStrategy::WithDefaults => meta.required || meta.default.is_some(),
                TemplateStrategy::Everything => true,
            };
            if !selected {
                return;
            }
            let key = match path.last() {
                Some(confy_core::model::node::Seg::Key(k)) => k.clone(),
                _ => return,
            };
            let value = match (&meta.default, presence) {
                (Some(v), _) if strategy != TemplateStrategy::RequiredOnly => v.to_string(),
                (_, Presence::Set { literal }) | (_, Presence::Invalid { literal, .. }) => {
                    literal.clone()
                }
                _ => empty_of(meta),
            };
            if comments == CommentPolicy::Allow {
                for text in [Some(&meta.node.title), meta.node.description.as_ref()]
                    .into_iter()
                    .flatten()
                    .filter(|t| !t.is_empty())
                {
                    out.push(format!("{} {text}", comment_prefix(fmt)));
                }
            }
            out.push(match fmt {
                DocFormat::Json => format!("  \"{key}\": {value}"),
                DocFormat::Yaml => format!("{key}: {value}"),
                DocFormat::Toml => format!("{key} = {value}"),
            });
        }
        _ => {}
    }
}

/// A placeholder for a Field with no value and no default: the type's empty literal.
fn empty_of(meta: &confyg_form::ir::FieldMeta) -> String {
    match meta.examples.first() {
        Some(v) => v.to_string(),
        None => "\"\"".to_owned(),
    }
}

fn comment_prefix(fmt: DocFormat) -> &'static str {
    match fmt {
        DocFormat::Json => "//",
        _ => "#",
    }
}
