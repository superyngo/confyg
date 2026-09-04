//! The session's contract: no I/O, one snapshot per dispatch, one undo entry per intent.

use confy_core::model::document::DocFormat;
use confy_core::model::node::{Path, Seg};
use confy_core::schema::types::SchemaSource;
use confyg_form::ir::FormNode;
use confyg_session::lower::SetterIntent;
use confyg_session::session::{Request, Session, SessionCommand};
use serde_json::{json, Value};

fn key(k: &str) -> Path {
    vec![Seg::Key(k.into())]
}

fn cmd(c: SessionCommand) -> Request {
    Request::Command(c)
}

fn intent(i: SetterIntent) -> Request {
    Request::Intent(i)
}

fn schema_host_port() -> Value {
    json!({"properties":{"host":{"type":"string"},"port":{"type":"integer"}}})
}

fn session_with(schema: Value, text: &str) -> Session {
    let mut s = Session::new();
    s.dispatch(cmd(SessionCommand::Open {
        text: text.into(),
        fmt: DocFormat::Toml,
        path: Some("a.toml".into()),
    }));
    s.dispatch(cmd(SessionCommand::LoadSchema {
        source: SchemaSource::Local("a.schema.json".into()),
        text: schema.to_string(),
    }));
    s
}

#[test]
fn opening_without_a_schema_emits_a_fetch_request_from_the_hint() {
    let mut s = Session::new();
    let snap = s.dispatch(cmd(SessionCommand::Open {
        text: "#:schema https://example.com/a.json\nhost = \"a\"\n".into(),
        fmt: DocFormat::Toml,
        path: Some("a.toml".into()),
    }));
    assert!(
        snap.fetch.is_some(),
        "confyg-session performs no I/O (design §6)"
    );
    assert!(matches!(
        snap.ir,
        FormNode::Unknown { .. } | FormNode::Group { .. }
    ));
}

#[test]
fn with_no_schema_at_all_the_document_is_one_unknown_group() {
    let mut s = Session::new();
    let snap = s.dispatch(cmd(SessionCommand::Open {
        text: "host = \"a\"\n".into(),
        fmt: DocFormat::Toml,
        path: None,
    }));
    assert!(
        matches!(snap.ir, FormNode::Unknown { .. }),
        "design §6 step 4"
    );
    assert_eq!(snap.text, "host = \"a\"\n", "every byte is still there");
}

#[test]
fn loading_the_schema_clears_the_fetch_and_projects_a_form() {
    let mut s = session_with(schema_host_port(), "host = \"a\"\n");
    let snap = s.dispatch(cmd(SessionCommand::Save));
    assert!(snap.fetch.is_none());
    assert!(matches!(snap.ir, FormNode::Group { .. }));
}

#[test]
fn undo_is_one_entry_per_committed_intent() {
    let mut s = session_with(schema_host_port(), "port = 80\n");
    assert!(!s.dispatch(cmd(SessionCommand::Save)).can_undo);
    s.dispatch(intent(SetterIntent::SetValue {
        path: key("port"),
        value: json!(1),
    }));
    s.dispatch(intent(SetterIntent::SetValue {
        path: key("port"),
        value: json!(2),
    }));
    assert_eq!(s.dispatch(cmd(SessionCommand::Undo)).text, "port = 1\n");
    assert_eq!(s.dispatch(cmd(SessionCommand::Undo)).text, "port = 80\n");
    let redone = s.dispatch(cmd(SessionCommand::Redo));
    assert_eq!(redone.text, "port = 1\n");
    assert!(redone.can_undo && redone.can_redo);
}

#[test]
fn a_refused_intent_costs_no_undo_entry_and_reports_a_notice() {
    let s2 = json!({"required":["host"],"properties":{"host":{"type":"string"}}});
    let mut s = session_with(s2, "host = \"a\"\n");
    let snap = s.dispatch(intent(SetterIntent::Unset { path: key("host") }));
    assert_eq!(snap.text, "host = \"a\"\n");
    assert!(!snap.can_undo);
    assert!(snap
        .notices
        .iter()
        .any(|n| n.code == "session.intent.refused"));
}

#[test]
fn convert_format_re_emits_the_whole_document() {
    let mut s = session_with(schema_host_port(), "host = \"a\"\n");
    let out = s
        .dispatch(cmd(SessionCommand::ConvertFormat(DocFormat::Json)))
        .text;
    assert_eq!(
        serde_json::from_str::<Value>(&out).expect("valid JSON")["host"],
        "a"
    );
    assert_eq!(
        s.dispatch(cmd(SessionCommand::Undo)).text,
        "host = \"a\"\n",
        "a conversion is one undo step like any other"
    );
}

#[test]
fn the_snapshot_serializes_for_the_ffi_boundary() {
    let mut s = session_with(schema_host_port(), "port = 80\n");
    let snap = s.dispatch(cmd(SessionCommand::Save));
    let json = serde_json::to_value(&snap).expect("snapshot is serializable");
    for field in ["ir", "summary", "text", "notices", "canUndo", "canRedo"] {
        assert!(json.get(field).is_some(), "missing {field}");
    }
}
