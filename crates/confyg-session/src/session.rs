//! Design §6: the session. One `dispatch` in, one `SetterSnapshot` out.
//!
//! The session performs **no I/O**: when a Document names a Schema it cannot resolve itself, the
//! snapshot carries a `SchemaFetchRequest` and the host fetches it. That is what keeps the same
//! code honest in a browser, a terminal and an extension host.
//!
//! Undo is a **full-text snapshot ring**, one entry per committed intent, ported from confy's
//! `session::undo_redo`: with a lossless CST the whole text is the cheapest correct undo unit,
//! and it cannot drift from the tree the way a replayed mutation log can.

use confy_core::model::any_doc::AnyDocument;
use confy_core::model::convert::convert;
use confy_core::model::document::{ConfigDocument, DocFormat};
use confy_core::schema::hints::detect_hint;
use confy_core::schema::types::{SchemaSource, Violation};
use confyg_form::affordance::{Density, HostProfile};
use confyg_form::compile::project;
use confyg_form::ir::FormNode;
use confyg_form::notice::Notice;
use confyg_form::search::Hit;
use confyg_form::unknown::{summary, Summary, Validation};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::lower::{lower, SetterIntent};

/// Everything a host may ask of a session. Externally tagged, because the FFI boundary carries
/// it as JSON and an internal tag would collide with the command's own `kind`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Request {
    Intent(SetterIntent),
    Command(SessionCommand),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SessionCommand {
    Open {
        text: String,
        fmt: DocFormat,
        path: Option<String>,
    },
    Save,
    ConvertFormat(DocFormat),
    LoadSchema {
        source: SchemaSource,
        text: String,
    },
    Undo,
    Redo,
}

/// What the host must fetch, because the session will not.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SchemaFetchRequest {
    pub source: SchemaSource,
}

/// The only type that crosses the FFI boundary. `confyg-ffi` and `web/` consume exactly these
/// fields and nothing else.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetterSnapshot {
    pub ir: FormNode,
    pub summary: Summary,
    pub text: String,
    pub notices: Vec<Notice>,
    pub fetch: Option<SchemaFetchRequest>,
    pub can_undo: bool,
    pub can_redo: bool,
}

pub struct Session {
    doc: Option<AnyDocument>,
    schema: Option<Value>,
    host: HostProfile,
    path: Option<String>,
    pending_fetch: Option<SchemaFetchRequest>,
    notices: Vec<Notice>,
    /// Full-text snapshots carry their **Doc format**: a format conversion is an undo step like
    /// any other, and re-parsing TOML bytes as JSON would silently lose the document.
    undo: Vec<(String, DocFormat)>,
    redo: Vec<(String, DocFormat)>,
}

impl Default for Session {
    fn default() -> Self {
        Session::new()
    }
}

impl Session {
    pub fn new() -> Self {
        Session::with_host(HostProfile {
            can_mask: true,
            can_slide: true,
            can_filter_options: false,
            density: Density::Desktop,
        })
    }

    pub fn with_host(host: HostProfile) -> Self {
        Session {
            doc: None,
            schema: None,
            host,
            path: None,
            pending_fetch: None,
            notices: Vec::new(),
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }

    pub fn dispatch(&mut self, req: Request) -> SetterSnapshot {
        self.notices.clear();
        match req {
            Request::Command(cmd) => self.command(cmd),
            Request::Intent(intent) => self.intent(&intent),
        }
        self.snapshot()
    }

    fn command(&mut self, cmd: SessionCommand) {
        match cmd {
            SessionCommand::Open { text, fmt, path } => {
                self.path = path;
                self.undo.clear();
                self.redo.clear();
                self.pending_fetch =
                    detect_hint(&text, fmt).map(|source| SchemaFetchRequest { source });
                match AnyDocument::from_str_as(&text, fmt) {
                    Ok(doc) => self.doc = Some(doc),
                    Err(e) => self
                        .notices
                        .push(Notice::new("session.open.parse", e.to_string())),
                }
            }
            SessionCommand::LoadSchema { source, text } => match serde_json::from_str(&text) {
                Ok(schema) => {
                    self.schema = Some(schema);
                    self.pending_fetch = None;
                    let _ = source;
                }
                Err(e) => self
                    .notices
                    .push(Notice::new("session.schema.parse", e.to_string())),
            },
            SessionCommand::ConvertFormat(target) => {
                let Some(doc) = self.doc.as_ref() else { return };
                match convert(doc, target) {
                    Ok(result) => {
                        for w in result.warnings {
                            self.notices.push(Notice::new("session.convert.warning", w));
                        }
                        self.commit_text(&result.text, target);
                    }
                    Err(abort) => self
                        .notices
                        .push(Notice::new("session.convert.abort", format!("{abort:?}"))),
                }
            }
            SessionCommand::Undo => self.step(true),
            SessionCommand::Redo => self.step(false),
            // Saving is the host's I/O; the session only says what the bytes are, which every
            // snapshot already carries.
            SessionCommand::Save => {}
        }
    }

    fn intent(&mut self, intent: &SetterIntent) {
        let Some(doc) = self.doc.as_mut() else { return };
        let schema = self.schema.clone().unwrap_or(Value::Bool(true));
        let ir = project(&schema, Some(doc), &self.host);
        let before = doc.serialize();
        let prediction = predicted(intent, &ir.root);
        match lower(intent, &ir.root, doc, &schema) {
            Ok(muts) => {
                if muts.is_empty() {
                    return;
                }
                for m in muts {
                    if let Err(e) = doc.apply(m) {
                        self.notices
                            .push(Notice::new("session.mutate.failed", format!("{e:?}")));
                        return;
                    }
                }
                // One undo entry per *committed* intent, not per Mutation: a host asked for one
                // change and gets one step back.
                self.undo.push((before, doc.format()));
                self.redo.clear();
                self.check_postcondition(&prediction, &schema);
            }
            Err(refused) => self
                .notices
                .push(Notice::new("session.intent.refused", refused.reason)),
        }
    }

    /// The D9 guard. Upstream's failure mode is *success plus a structurally different
    /// document*, so the recompiled IR is compared against what the intent said it would do:
    /// a Notice carrying both shapes in release builds, a panic in tests.
    fn check_postcondition(&mut self, prediction: &Prediction, schema: &Value) {
        let after = project(schema, self.doc.as_ref(), &self.host);
        let got = observed(&after.root, &prediction.path);
        if got == prediction.expect {
            return;
        }
        let message = format!(
            "predicted {:?} at {:?}, recompiled to {:?}",
            prediction.expect, prediction.path, got
        );
        debug_assert!(false, "D9: {message}");
        self.notices
            .push(Notice::new("session.postcondition.mismatch", message));
    }

    fn step(&mut self, undo: bool) {
        let (from, to) = if undo {
            (&mut self.undo, &mut self.redo)
        } else {
            (&mut self.redo, &mut self.undo)
        };
        let Some((text, fmt)) = from.pop() else {
            return;
        };
        let Some(doc) = self.doc.as_ref() else { return };
        to.push((doc.serialize(), doc.format()));
        match AnyDocument::from_str_as(&text, fmt) {
            Ok(next) => self.doc = Some(next),
            Err(e) => self
                .notices
                .push(Notice::new("session.undo.parse", e.to_string())),
        }
    }

    /// Replace the whole Document — used by format conversion, which re-emits every byte.
    fn commit_text(&mut self, text: &str, fmt: DocFormat) {
        let before = self.doc.as_ref().map(|d| (d.serialize(), d.format()));
        match AnyDocument::from_str_as(text, fmt) {
            Ok(doc) => {
                if let Some(before) = before {
                    self.undo.push(before);
                    self.redo.clear();
                }
                self.doc = Some(doc);
            }
            Err(e) => self
                .notices
                .push(Notice::new("session.convert.parse", e.to_string())),
        }
    }

    /// Violations for a literal the user is still typing, from the **validator's own engine**.
    ///
    /// The alternative — re-implementing `pattern` host-side with a different regex flavour —
    /// makes a form warning that can disagree with a Violation, and `jsonschema` runs
    /// `fancy-regex`, which accepts lookarounds Rust's `regex` rejects (design §7).
    ///
    /// The probe is a *copy* of the document: nothing here is committed, so a live check can
    /// never write.
    pub fn check(&self, path: &confy_core::model::node::Path, literal: &str) -> Vec<Violation> {
        let (Some(doc), Some(schema)) = (self.doc.as_ref(), self.schema.as_ref()) else {
            return Vec::new();
        };
        let Ok(mut probe) = AnyDocument::from_str_as(&doc.serialize(), doc.format()) else {
            return Vec::new();
        };
        let key = match path.last() {
            Some(confy_core::model::node::Seg::Key(k)) => Some(k.clone()),
            _ => None,
        };
        // The literal arrives exactly as typed, so it may not be a valid value literal on its
        // own (`xyz` is a bare TOML word). Try it verbatim first — that is what a number or a
        // bool needs — then as a quoted string, which is what a half-typed text field is.
        let quoted = Value::String(literal.to_owned()).to_string();
        let mut applied = false;
        for candidate in [literal, quoted.as_str()] {
            let fragment = probe.scalar_fragment(key.as_deref(), candidate);
            if probe
                .apply(confy_core::model::document::Mutation::Replace {
                    path: path.clone(),
                    fragment,
                })
                .is_ok()
            {
                applied = true;
                break;
            }
        }
        if !applied {
            return Vec::new();
        }
        let compiled = project(schema, Some(&probe), &self.host);
        summary(&compiled.root, &compiled.state)
            .items
            .into_iter()
            .filter(|item| &item.path == path)
            .map(|item| Violation {
                path: item.path,
                pointer: String::new(),
                keyword: item.keyword,
                message: item.message,
                category: confy_core::schema::types::Category::Value,
            })
            .collect()
    }

    /// **Form search** over the document as it stands, answered by the compiler that owns the
    /// semantics (presentation §5.3).
    ///
    /// The session recompiles rather than publishing its IR. A public accessor would let any
    /// host read the compiled tree and re-derive form decisions locally, which is the one
    /// thing `web/` never does — every decision arrives in a `SetterSnapshot`.
    ///
    /// With no Schema there is no form to search, so the result is empty rather than a scan
    /// of the raw document: search matches **Form nodes**, not bytes.
    pub fn search(&self, query: &str) -> Vec<Hit> {
        let Some(schema) = self.schema.as_ref() else {
            return Vec::new();
        };
        let compiled = project(schema, self.doc.as_ref(), &self.host);
        confyg_form::search::search(&compiled.root, query)
    }

    fn snapshot(&self) -> SetterSnapshot {
        let text = self
            .doc
            .as_ref()
            .map(ConfigDocument::serialize)
            .unwrap_or_default();

        // Design §6 step 4: with no Schema at all, there is nothing to project a form from, so
        // the whole Document is one Unknown node — preserved, never reshaped.
        let (ir, summary, mut notices) = match (&self.schema, &self.doc) {
            (Some(schema), doc) => {
                let compiled = project(schema, doc.as_ref(), &self.host);
                let sum = summary(&compiled.root, &compiled.state);
                (compiled.root, sum, compiled.notices)
            }
            (None, _) => (
                FormNode::Unknown {
                    path: Vec::new(),
                    raw_preview: text.clone(),
                },
                Summary {
                    items: Vec::new(),
                    validation: Validation::Available,
                },
                Vec::new(),
            ),
        };
        notices.extend(self.notices.iter().cloned());

        SetterSnapshot {
            ir,
            summary,
            text,
            notices,
            fetch: self.pending_fetch.clone(),
            can_undo: !self.undo.is_empty(),
            can_redo: !self.redo.is_empty(),
        }
    }
}

/// What an intent says the form will look like after it is applied. Deliberately coarse: the
/// guard exists to catch *structural* surprises (a key written into the wrong container, an item
/// that never landed), not to re-specify the value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Shape {
    Present,
    Absent,
    Count(usize),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Prediction {
    pub path: confy_core::model::node::Path,
    pub expect: Shape,
}

/// The postcondition of each v0.1 intent, read off the IR *before* the write.
pub fn predicted(intent: &SetterIntent, before: &FormNode) -> Prediction {
    let path = intent.path().clone();
    let expect = match intent {
        // Writing the effective default removes the key: the default is written by absence.
        SetterIntent::SetValue { value, .. } => match field_default(before, &path) {
            Some(default) if &default == value => Shape::Absent,
            _ => Shape::Present,
        },
        SetterIntent::Unset { .. } => Shape::Absent,
        SetterIntent::AddRepeatItem { .. } => Shape::Count(repeat_len(before, &path) + 1),
        SetterIntent::RemoveRepeatItem { .. } => {
            Shape::Count(repeat_len(before, &path).saturating_sub(1))
        }
        SetterIntent::ToggleGroup { enable, .. } => {
            if *enable {
                Shape::Present
            } else {
                Shape::Absent
            }
        }
    };
    Prediction { path, expect }
}

/// The same shape, read off a compiled IR. A path that no longer resolves is `Absent`.
pub fn observed(ir: &FormNode, path: &confy_core::model::node::Path) -> Shape {
    match find_node(ir, path) {
        None => Shape::Absent,
        Some(FormNode::Field { presence, .. }) => match presence {
            confyg_form::ir::Presence::Absent { .. } => Shape::Absent,
            _ => Shape::Present,
        },
        Some(FormNode::Group { occupancy, .. }) => {
            if *occupancy == confyg_form::ir::Occupancy::Absent {
                Shape::Absent
            } else {
                Shape::Present
            }
        }
        Some(FormNode::Repeat { items, .. }) => Shape::Count(items.len()),
        Some(_) => Shape::Present,
    }
}

fn field_default(ir: &FormNode, path: &confy_core::model::node::Path) -> Option<Value> {
    match find_node(ir, path) {
        Some(FormNode::Field { meta, .. }) => meta.default.clone(),
        _ => None,
    }
}

fn repeat_len(ir: &FormNode, path: &confy_core::model::node::Path) -> usize {
    match find_node(ir, path) {
        Some(FormNode::Repeat { items, .. }) => items.len(),
        _ => 0,
    }
}

fn find_node<'a>(ir: &'a FormNode, path: &confy_core::model::node::Path) -> Option<&'a FormNode> {
    if confyg_form::compile::path_of(ir) == path {
        return Some(ir);
    }
    let children: &[FormNode] = match ir {
        FormNode::Group { children, .. } => children,
        FormNode::Repeat { items, .. } => items,
        _ => return None,
    };
    children.iter().find_map(|c| find_node(c, path))
}
