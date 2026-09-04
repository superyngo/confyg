//! Verification item 2, the round-trip matrix: `(schema, document, [intent]) → expected bytes`.
//!
//! Every case runs in all three **Doc formats**, and the comment-interleaved variant is
//! mandatory in each: a lowering that reports success while misplacing text is the failure mode
//! this file exists to catch.

use confy_core::model::any_doc::AnyDocument;
use confy_core::model::document::{ConfigDocument, DocFormat};
use confy_core::model::node::{Path, Seg};
use confy_core::schema::types::Violation;
use confyg_form::affordance::{Density, HostProfile};
use confyg_form::compile::project;
use confyg_form::ir::TemplateRef;
use confyg_session::fragment::{fragment, Emission};
use confyg_session::lower::{lower, Refused, SetterIntent};
use serde_json::{json, Value};

fn host() -> HostProfile {
    HostProfile {
        can_mask: true,
        can_slide: true,
        can_filter_options: false,
        density: Density::Desktop,
    }
}

pub fn key(k: &str) -> Path {
    vec![Seg::Key(k.into())]
}

/// Project, lower, apply — the whole session write path — and return the bytes.
fn apply_all(schema: &Value, src: &str, fmt: DocFormat, intents: &[SetterIntent]) -> String {
    let mut doc = AnyDocument::from_str_as(src, fmt).expect("parse");
    for intent in intents {
        let ir = project(schema, Some(&doc), &host());
        let muts = lower(intent, &ir.root, &doc, schema).expect("host offered an ungated intent");
        for m in muts {
            doc.apply(m).expect("mutation");
        }
    }
    doc.serialize()
}

fn lower_err(schema: &Value, src: &str, fmt: DocFormat, intent: SetterIntent) -> Option<Refused> {
    let doc = AnyDocument::from_str_as(src, fmt).expect("parse");
    let ir = project(schema, Some(&doc), &host());
    lower(&intent, &ir.root, &doc, schema).err()
}

fn violations_after(schema: &Value, src: &str, fmt: DocFormat) -> Vec<Violation> {
    let doc = AnyDocument::from_str_as(src, fmt).expect("parse");
    let ir = project(schema, Some(&doc), &host());
    fn walk(n: &confyg_form::ir::FormNode, out: &mut Vec<Violation>) {
        use confyg_form::ir::FormNode;
        match n {
            FormNode::Field { meta, .. } => out.extend(meta.node.violations.clone()),
            FormNode::Group { meta, children, .. } => {
                out.extend(meta.violations.clone());
                children.iter().for_each(|c| walk(c, out));
            }
            _ => {}
        }
    }
    let mut out = Vec::new();
    walk(&ir.root, &mut out);
    out
}

fn schema_host_port() -> Value {
    json!({"properties":{"host":{"type":"string"},"port":{"type":"integer"}}})
}

/// One test body, three formats. Each format brings its own source and expected bytes, because
/// the *bytes* are format-specific while the lowering under test is not.
macro_rules! matrix {
    ($name:ident, |$fmt:ident, $src:ident, $want:ident| $body:block, $($f:ident: $s:expr => $w:expr),+ $(,)?) => {
        #[test]
        fn $name() {
            $({
                let $fmt = DocFormat::$f;
                let $src: &str = $s;
                let $want: &str = $w;
                $body
            })+
        }
    };
}

matrix!(
    set_value_replaces_and_leaves_every_other_byte_alone,
    |fmt, src, want| {
        let out = apply_all(
            &schema_host_port(),
            src,
            fmt,
            &[SetterIntent::SetValue {
                path: key("port"),
                value: json!(8080),
            }],
        );
        assert_eq!(out, want, "{fmt:?}");
    },
    Toml: "# lead\nhost = \"a\"  # trail\nport = 80\n"
        => "# lead\nhost = \"a\"  # trail\nport = 8080\n",
    Json: "{\n  // lead\n  \"host\": \"a\",\n  \"port\": 80\n}\n"
        => "{\n  // lead\n  \"host\": \"a\",\n  \"port\": 8080\n}\n",
    Yaml: "# lead\nhost: a  # trail\nport: 80\n"
        => "# lead\nhost: a  # trail\nport: 8080\n",
);

matrix!(
    setting_a_value_equal_to_the_default_deletes_the_key,
    |fmt, src, want| {
        let s = json!({"properties":{"level":{"type":"string","default":"info"}}});
        let out = apply_all(
            &s,
            src,
            fmt,
            &[SetterIntent::SetValue {
                path: key("level"),
                value: json!("info"),
            }],
        );
        assert_eq!(
            out, want,
            "{fmt:?}: Minimal write is a correctness rule (ADR 0003)"
        );
    },
    Toml: "level = \"debug\"\n" => "",
    Json: "{\n  \"level\": \"debug\"\n}\n" => "{\n}\n",
    Yaml: "level: debug\n" => "",
);

matrix!(
    an_invalid_value_is_written_and_warned_about_never_refused,
    |fmt, src, want| {
        let s = json!({"properties":{"port":{"type":"integer","minimum":1}}});
        let out = apply_all(
            &s,
            src,
            fmt,
            &[SetterIntent::SetValue {
                path: key("port"),
                value: json!(-5),
            }],
        );
        assert_eq!(out, want, "{fmt:?}: Soft constraint");
        assert_eq!(violations_after(&s, &out, fmt).len(), 1, "{fmt:?}");
    },
    Toml: "port = 80\n" => "port = -5\n",
    Json: "{\n  \"port\": 80\n}\n" => "{\n  \"port\": -5\n}\n",
    Yaml: "port: 80\n" => "port: -5\n",
);

matrix!(
    unset_removes_the_key_but_is_not_offered_when_required,
    |fmt, src, want| {
        let optional = json!({"properties":{"host":{"type":"string"}}});
        let out = apply_all(
            &optional,
            src,
            fmt,
            &[SetterIntent::Unset { path: key("host") }],
        );
        assert_eq!(out, want, "{fmt:?}");

        let required = json!({"required":["host"],"properties":{"host":{"type":"string"}}});
        assert!(
            lower_err(&required, src, fmt, SetterIntent::Unset { path: key("host") }).is_some(),
            "{fmt:?}: a required Field has no Unset affordance (design §5)"
        );
    },
    Toml: "host = \"a\"\n" => "",
    Json: "{\n  \"host\": \"a\"\n}\n" => "{\n}\n",
    Yaml: "host: a\n" => "",
);

matrix!(
    a_missing_key_is_inserted_at_its_schema_position,
    |fmt, src, want| {
        let s = json!({"properties":{"host":{"type":"string"},"port":{"type":"integer"},
                                     "ca":{"type":"string"}}});
        let out = apply_all(
            &s,
            src,
            fmt,
            &[SetterIntent::SetValue {
                path: key("port"),
                value: json!(80),
            }],
        );
        assert_eq!(out, want, "{fmt:?}: never appended blindly");
    },
    Toml: "host = \"a\"\nca = \"c\"\n" => "host = \"a\"\nport = 80\nca = \"c\"\n",
    Json: "{\n  \"host\": \"a\",\n  \"ca\": \"c\"\n}\n"
        => "{\n  \"host\": \"a\",\n  \"port\": 80,\n  \"ca\": \"c\"\n}\n",
    Yaml: "host: a\nca: c\n" => "host: a\nport: 80\nca: c\n",
);

matrix!(
    an_insert_lands_after_an_interleaved_comment_not_inside_it,
    |fmt, src, want| {
        let s = json!({"properties":{"host":{"type":"string"},"port":{"type":"integer"},
                                     "ca":{"type":"string"}}});
        let out = apply_all(
            &s,
            src,
            fmt,
            &[SetterIntent::SetValue {
                path: key("port"),
                value: json!(80),
            }],
        );
        assert_eq!(out, want, "{fmt:?}: D7");
    },
    Toml: "host = \"a\"\n# about ca\nca = \"c\"\n"
        => "host = \"a\"\nport = 80\n# about ca\nca = \"c\"\n",
    Json: "{\n  \"host\": \"a\",\n  // about ca\n  \"ca\": \"c\"\n}\n"
        => "{\n  \"host\": \"a\",\n  \"port\": 80,\n  // about ca\n  \"ca\": \"c\"\n}\n",
    Yaml: "host: a\n# about ca\nca: c\n"
        => "host: a\nport: 80\n# about ca\nca: c\n",
);

// ── Task 11: collections ────────────────────────────────────────────────────────────────────

fn servers_schema() -> Value {
    json!({"properties":{"servers":{"type":"array","maxItems":2,
        "items":{"type":"object","properties":{"host":{"type":"string"}}}}}})
}

matrix!(
    the_first_item_of_an_absent_collection_is_a_different_mutation_from_the_second,
    |fmt, src, want| {
        let s = servers_schema();
        let add = [SetterIntent::AddRepeatItem {
            path: key("servers"),
        }];
        let after_first = apply_all(&s, src, fmt, &add);
        let after_second = apply_all(&s, &after_first, fmt, &add);
        assert_eq!(
            format!("{after_first}|{after_second}"),
            want,
            "{fmt:?}: absent-parent lowering, then D2's headerless insert at the collection Path"
        );
    },
    Toml: "" => "[[servers]]\nhost = \"\"\n|[[servers]]\nhost = \"\"\n[[servers]]\nhost = \"\"\n",
    Json: "{}\n" => "{ \"servers\": [{ \"host\": \"\" }] }\n|{ \"servers\": [{ \"host\": \"\" }, { \"host\": \"\" }] }\n",
    Yaml: "" => "servers:\n  - host: \"\"\n|servers:\n  - host: \"\"\n  - host: \"\"\n",
);

matrix!(
    add_is_not_offered_at_max_items_and_remove_not_below_min,
    |fmt, src, _want| {
        let s = json!({"properties":{"a":{"type":"array","maxItems":1,"minItems":1,
                                          "items":{"type":"string"}}}});
        assert!(
            lower_err(
                &s,
                src,
                fmt,
                SetterIntent::AddRepeatItem { path: key("a") }
            )
            .is_some(),
            "{fmt:?}: maxItems"
        );
        assert!(
            lower_err(
                &s,
                src,
                fmt,
                SetterIntent::RemoveRepeatItem {
                    path: key("a"),
                    index: 0
                }
            )
            .is_some(),
            "{fmt:?}: minItems"
        );
    },
    Toml: "a = [\"x\"]\n" => "",
    Json: "{ \"a\": [\"x\"] }\n" => "",
    Yaml: "a:\n  - x\n" => "",
);

matrix!(
    toggling_an_optional_group_writes_its_template_and_removes_the_whole_section,
    |fmt, src, want| {
        let s = json!({"properties":{"host":{"type":"string"},"tls":{"type":"object",
            "properties":{"on":{"type":"boolean","default":false},"ca":{"type":"string"}}}}});
        let on = apply_all(
            &s,
            src,
            fmt,
            &[SetterIntent::ToggleGroup {
                path: key("tls"),
                enable: true,
            }],
        );
        let off = apply_all(
            &s,
            &on,
            fmt,
            &[SetterIntent::ToggleGroup {
                path: key("tls"),
                enable: false,
            }],
        );
        assert_eq!(
            format!("{on}|{off}"),
            want,
            "{fmt:?}: the default-valued `on` is never written (Minimal write)"
        );
    },
    Toml: "host = \"a\"\n" => "host = \"a\"\n[tls]\nca = \"\"\n|host = \"a\"\n",
    Json: "{ \"host\": \"a\" }\n"
        => "{ \"host\": \"a\", \"tls\": { \"ca\": \"\" } }\n|{ \"host\": \"a\" }\n",
    Yaml: "host: a\n" => "host: a\ntls:\n  ca: \"\"\n|host: a\n",
);

#[test]
fn a_header_fragment_is_never_emitted_at_the_collection_path() {
    // Guards the D2 asymmetry: assert the Emission choice, not the engine's tolerance.
    let f = fragment(
        &servers_schema(),
        &TemplateRef("#/properties/servers/items".into()),
        DocFormat::Toml,
        Emission::Headerless,
    );
    assert!(!f.starts_with("[["), "got {f:?}");
    let h = fragment(
        &servers_schema(),
        &TemplateRef("#/properties/servers/items".into()),
        DocFormat::Toml,
        Emission::HeaderBearing,
    );
    assert!(h.starts_with("[[servers]]"), "got {h:?}");
}
