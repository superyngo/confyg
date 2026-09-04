//! D9: upstream's `Insert` reports success even when it wrote a structurally different
//! document. Every v0.1 intent must therefore recompile to the shape it predicted.

use confy_core::model::document::DocFormat;
use confy_core::model::node::{Path, Seg};
use confy_core::schema::types::SchemaSource;
use confyg_form::affordance::{Density, HostProfile};
use confyg_form::compile::project;
use confyg_form::ir::FormNode;
use confyg_session::lower::SetterIntent;
use confyg_session::session::{observed, predicted, Request, Session, SessionCommand};
use serde_json::{json, Value};

fn key(k: &str) -> Path {
    vec![Seg::Key(k.into())]
}

fn host() -> HostProfile {
    HostProfile {
        can_mask: true,
        can_slide: true,
        can_filter_options: false,
        density: Density::Desktop,
    }
}

fn session_with(schema: Value, src: &str, fmt: DocFormat) -> Session {
    let mut s = Session::new();
    s.dispatch(Request::Command(SessionCommand::Open {
        text: src.into(),
        fmt,
        path: Some("a".into()),
    }));
    s.dispatch(Request::Command(SessionCommand::LoadSchema {
        source: SchemaSource::Local("a.schema.json".into()),
        text: schema.to_string(),
    }));
    s
}

fn before_ir(schema: &Value, src: &str, fmt: DocFormat) -> FormNode {
    let doc = confy_core::model::any_doc::AnyDocument::from_str_as(src, fmt).expect("parse");
    project(schema, Some(&doc), &host()).root
}

/// Every intent v0.1 ships, each with a document that makes it legal.
fn every_v0_1_intent_case() -> Vec<(Value, &'static str, SetterIntent)> {
    let servers = json!({"properties":{"servers":{"type":"array",
        "items":{"type":"object","properties":{"host":{"type":"string"}}}}}});
    let tls = json!({"properties":{"host":{"type":"string"},
        "tls":{"type":"object","properties":{"ca":{"type":"string"}}}}});
    vec![
        (
            json!({"properties":{"port":{"type":"integer"}}}),
            "port = 80\n",
            SetterIntent::SetValue {
                path: key("port"),
                value: json!(8080),
            },
        ),
        (
            json!({"properties":{"port":{"type":"integer"}}}),
            "",
            SetterIntent::SetValue {
                path: key("port"),
                value: json!(8080),
            },
        ),
        (
            json!({"properties":{"level":{"type":"string","default":"info"}}}),
            "level = \"debug\"\n",
            SetterIntent::SetValue {
                path: key("level"),
                value: json!("info"),
            },
        ),
        (
            json!({"properties":{"host":{"type":"string"}}}),
            "host = \"a\"\n",
            SetterIntent::Unset { path: key("host") },
        ),
        (
            servers.clone(),
            "",
            SetterIntent::AddRepeatItem {
                path: key("servers"),
            },
        ),
        (
            servers.clone(),
            "[[servers]]\nhost = \"a\"\n",
            SetterIntent::AddRepeatItem {
                path: key("servers"),
            },
        ),
        (
            servers,
            "[[servers]]\nhost = \"a\"\n",
            SetterIntent::RemoveRepeatItem {
                path: key("servers"),
                index: 0,
            },
        ),
        (
            tls.clone(),
            "host = \"a\"\n",
            SetterIntent::ToggleGroup {
                path: key("tls"),
                enable: true,
            },
        ),
        (
            tls,
            "host = \"a\"\n[tls]\nca = \"c\"\n",
            SetterIntent::ToggleGroup {
                path: key("tls"),
                enable: false,
            },
        ),
    ]
}

#[test]
fn every_intent_recompiles_to_the_shape_it_predicted() {
    for (schema, src, intent) in every_v0_1_intent_case() {
        let mut s = session_with(schema.clone(), src, DocFormat::Toml);
        // `dispatch` runs the same comparison internally and panics in test builds; asserting it
        // here as well names the failing case.
        let snap = s.dispatch(Request::Intent(intent.clone()));
        let want = predicted(&intent, &before_ir(&schema, src, DocFormat::Toml));
        assert_eq!(
            observed(&snap.ir, intent.path()),
            want.expect,
            "D9: {intent:?} wrote a shape it did not predict"
        );
        assert!(
            !snap
                .notices
                .iter()
                .any(|n| n.code == "session.postcondition.mismatch"),
            "D9 guard fired for {intent:?}"
        );
    }
}

#[test]
fn a_generic_placeholder_key_never_appears() {
    for (schema, src, intent) in every_v0_1_intent_case() {
        let mut s = session_with(schema, src, DocFormat::Toml);
        let text = s.dispatch(Request::Intent(intent.clone())).text;
        for generic in ["placeholder", "__elem__", "new_key", "key1"] {
            assert!(
                !text.contains(generic),
                "every Insert passes an explicit suggested_key: {intent:?} wrote {text:?}"
            );
        }
    }
}
