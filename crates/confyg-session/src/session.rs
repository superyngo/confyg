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
use confy_core::schema::types::SchemaSource;
use confyg_form::affordance::{Density, HostProfile};
use confyg_form::compile::project;
use confyg_form::ir::FormNode;
use confyg_form::notice::Notice;
use confyg_form::unknown::{summary, Summary, Validation};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::lower::{lower, SetterIntent};

/// Everything a host may ask of a session.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Request {
    Intent(SetterIntent),
    Command(SessionCommand),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
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
            }
            Err(refused) => self
                .notices
                .push(Notice::new("session.intent.refused", refused.reason)),
        }
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
