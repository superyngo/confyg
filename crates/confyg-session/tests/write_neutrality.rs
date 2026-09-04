//! Verification item 6: **Write-neutrality** (ADR 0004). Presentation input — the `x-confyg`
//! Annotation and the Host capability profile — decides how a form *looks*. It may never change
//! a single byte confyg writes.
//!
//! This test's job is to fail the day someone adds a presentation feature that writes. If it
//! fails, fix the leak: a leak found here is a design violation, not a test bug.

use confy_core::model::any_doc::AnyDocument;
use confy_core::model::document::{ConfigDocument, DocFormat};
use confy_core::model::node::{Path, Seg};
use confyg_form::affordance::{Density, HostProfile};
use confyg_form::compile::project;
use confyg_session::lower::{lower, SetterIntent};
use serde_json::{json, Value};

fn key(k: &str) -> Path {
    vec![Seg::Key(k.into())]
}

/// Full, no-mask, no-slide, filtering, and a TUI-ish profile that can do none of it.
fn host_profiles() -> Vec<HostProfile> {
    vec![
        HostProfile {
            can_mask: true,
            can_slide: true,
            can_filter_options: true,
            density: Density::Desktop,
        },
        HostProfile {
            can_mask: false,
            can_slide: true,
            can_filter_options: false,
            density: Density::Desktop,
        },
        HostProfile {
            can_mask: true,
            can_slide: false,
            can_filter_options: false,
            density: Density::Touch,
        },
        HostProfile {
            can_mask: false,
            can_slide: false,
            can_filter_options: false,
            density: Density::Phone,
        },
    ]
}

/// The same Schema decorated with every vocabulary member that could plausibly leak.
fn presentations() -> Vec<Value> {
    vec![
        json!({}),
        json!({"affordance":"text"}),
        json!({"affordance":"filterable-menu"}),
        json!({"order":9}),
        json!({"unit":"MiB"}),
        json!({"collapsed":true}),
        json!({"demoted":true}),
        json!({"label":"X"}),
        json!({"help":"Y"}),
        json!({"labelFrom":"name"}),
        json!({"optionLabels":{"info":"Informational"}}),
    ]
}

struct Case {
    schema: Value,
    /// The same logical document per format: the bytes differ, the lowering under test does not.
    src: [&'static str; 3],
    intents: Vec<SetterIntent>,
}

fn src_of(case: &Case, fmt: DocFormat) -> &'static str {
    match fmt {
        DocFormat::Toml => case.src[0],
        DocFormat::Json => case.src[1],
        DocFormat::Yaml => case.src[2],
    }
}

/// Every v0.1 intent, in a document that makes it legal. The Annotation is applied to every
/// property of the case's Schema, so no decorated node is left untested.
fn every_v0_1_intent_case() -> Vec<Case> {
    let scalars = json!({"properties":{
        "host":{"type":"string"},
        "port":{"type":"integer","minimum":1,"maximum":65535},
        "level":{"type":"string","enum":["info","warn"],"default":"info"},
        "secret":{"type":"string","writeOnly":true}}});
    let servers = json!({"properties":{"servers":{"type":"array",
        "items":{"type":"object","properties":{"name":{"type":"string"}}}}}});
    let tls = json!({"properties":{"host":{"type":"string"},
        "tls":{"type":"object","properties":{"ca":{"type":"string"}}}}});
    vec![
        Case {
            schema: scalars.clone(),
            src: [
                "host = \"a\"\n# about port\nport = 80\n",
                "{\n  \"host\": \"a\",\n  // about port\n  \"port\": 80\n}\n",
                "host: a\n# about port\nport: 80\n",
            ],
            intents: vec![
                SetterIntent::SetValue {
                    path: key("port"),
                    value: json!(8080),
                },
                SetterIntent::SetValue {
                    path: key("level"),
                    value: json!("warn"),
                },
                SetterIntent::SetValue {
                    path: key("secret"),
                    value: json!("s"),
                },
                SetterIntent::Unset { path: key("host") },
            ],
        },
        Case {
            schema: servers.clone(),
            src: ["", "{}\n", ""],
            intents: vec![
                SetterIntent::AddRepeatItem {
                    path: key("servers"),
                },
                SetterIntent::AddRepeatItem {
                    path: key("servers"),
                },
                SetterIntent::RemoveRepeatItem {
                    path: key("servers"),
                    index: 0,
                },
            ],
        },
        Case {
            schema: tls,
            src: ["host = \"a\"\n", "{ \"host\": \"a\" }\n", "host: a\n"],
            intents: vec![
                SetterIntent::ToggleGroup {
                    path: key("tls"),
                    enable: true,
                },
                SetterIntent::SetValue {
                    path: vec![Seg::Key("tls".into()), Seg::Key("ca".into())],
                    value: json!("c"),
                },
                SetterIntent::ToggleGroup {
                    path: key("tls"),
                    enable: false,
                },
            ],
        },
    ]
}

/// Decorate every property of `schema` with the `x-confyg` Annotation `p`.
fn decorate(schema: &Value, p: &Value) -> Value {
    let mut out = schema.clone();
    if p.as_object().is_some_and(|o| o.is_empty()) {
        return out;
    }
    fn walk(node: &mut Value, p: &Value) {
        if let Some(props) = node.get_mut("properties").and_then(Value::as_object_mut) {
            for (_, sub) in props.iter_mut() {
                sub["x-confyg"] = p.clone();
                walk(sub, p);
            }
        }
        if let Some(items) = node.get_mut("items") {
            items["x-confyg"] = p.clone();
            walk(items, p);
        }
    }
    walk(&mut out, p);
    out
}

/// Run a case end to end and return the bytes. A refused intent is recorded rather than
/// ignored, so "presentation changed what was *offered*" also shows up as a difference.
fn run(case: &Case, fmt: DocFormat, presentation: &Value, host: &HostProfile) -> String {
    let schema = decorate(&case.schema, presentation);
    let mut doc = AnyDocument::from_str_as(src_of(case, fmt), fmt).expect("parse");
    let mut log = String::new();
    for intent in &case.intents {
        let ir = project(&schema, Some(&doc), host);
        match lower(intent, &ir.root, &doc, &schema) {
            Ok(muts) => {
                for m in muts {
                    match doc.apply(m) {
                        Ok(_) => {}
                        Err(e) => log.push_str(&format!("[apply {e:?}]")),
                    }
                }
            }
            Err(r) => log.push_str(&format!("[refused {}]", r.reason)),
        }
    }
    format!("{}{log}", doc.serialize())
}

#[test]
fn presentation_can_never_reach_the_bytes() {
    for fmt in [DocFormat::Toml, DocFormat::Json, DocFormat::Yaml] {
        for (i, case) in every_v0_1_intent_case().iter().enumerate() {
            let baseline = run(case, fmt, &json!({}), &host_profiles()[0]);
            for p in presentations() {
                for h in host_profiles() {
                    assert_eq!(
                        run(case, fmt, &p, &h),
                        baseline,
                        "Write-neutrality broken by {p} on {h:?} in {fmt:?} case {i} (ADR 0004)"
                    );
                }
            }
        }
    }
}
