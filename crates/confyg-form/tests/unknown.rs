use confyg_form::affordance::{Density, HostProfile};
use confyg_form::compile::{key_of, project, Compiled};
use confyg_form::ir::FormNode;
use confyg_form::unknown::{summary, Validation};

use confy_core::model::any_doc::AnyDocument;
use confy_core::model::document::DocFormat;
use confy_core::schema::types::Violation;

fn host() -> HostProfile {
    HostProfile {
        can_mask: true,
        can_slide: true,
        can_filter_options: false,
        density: Density::Desktop,
    }
}

fn parse_toml(src: &str) -> AnyDocument {
    AnyDocument::from_str_as(src, DocFormat::Toml).expect("parse")
}

fn unknown_keys(root: &FormNode) -> Vec<String> {
    let FormNode::Group { children, .. } = root else {
        panic!("expected a Group")
    };
    children
        .iter()
        .filter(|c| matches!(c, FormNode::Unknown { .. }))
        .map(|c| key_of(c).unwrap_or_default().to_owned())
        .collect()
}

fn root_violations(root: &FormNode) -> &[Violation] {
    match root {
        FormNode::Group { meta, .. } => &meta.violations,
        other => panic!("expected a Group root, got {other:?}"),
    }
}

fn unknown_notice_names_key(c: &Compiled, key: &str) -> bool {
    c.notices
        .iter()
        .any(|n| n.code == "form.unknown.key" && n.message.contains(key))
}

#[test]
fn an_extra_key_under_open_additional_properties_is_a_notice_not_a_violation() {
    let s = serde_json::json!({"properties":{"a":{"type":"string"}}});
    let c = project(&s, Some(&parse_toml("a = \"x\"\nz = 1\n")), &host());
    assert_eq!(unknown_keys(&c.root), ["z"]);
    assert!(
        summary(&c.root, &c.state).items.is_empty(),
        "confyg does not fabricate failures (design §7 B18)"
    );
    assert_eq!(c.notices.len(), 1);
    assert!(unknown_notice_names_key(&c, "z"));
}

#[test]
fn closed_additional_properties_keeps_the_validator_s_container_violation() {
    let s = serde_json::json!({"additionalProperties": false,
                               "properties":{"a":{"type":"string"}}});
    let c = project(&s, Some(&parse_toml("a = \"x\"\nz = 1\n")), &host());
    assert_eq!(
        root_violations(&c.root).len(),
        1,
        "the message names the key; the Violation belongs to the container"
    );
    assert!(unknown_notice_names_key(&c, "z"));
}

#[test]
fn an_unknown_key_lands_at_the_end_of_its_own_parent() {
    let s = serde_json::json!({"properties":{
        "a":{"type":"string"},
        "sub":{"type":"object","properties":{"b":{"type":"string"}}}}});
    let doc = parse_toml("a = \"x\"\n[sub]\nb = \"y\"\nextra = 1\n");
    let c = project(&s, Some(&doc), &host());
    assert!(unknown_keys(&c.root).is_empty(), "extra is not the root's");
    let FormNode::Group { children, .. } = &c.root else {
        panic!()
    };
    let sub = children
        .iter()
        .find(|c| key_of(c) == Some("sub"))
        .expect("sub");
    assert_eq!(unknown_keys(sub), ["extra"]);
}

#[test]
fn a_violation_summary_lists_every_problem_depth_first() {
    let s = serde_json::json!({"properties":{
        "port":{"type":"integer"},
        "sub":{"type":"object","properties":{"n":{"type":"integer"}}}}});
    let doc = parse_toml("port = \"nope\"\n[sub]\nn = \"also nope\"\n");
    let c = project(&s, Some(&doc), &host());
    let sum = summary(&c.root, &c.state);
    assert_eq!(sum.items.len(), 2);
    assert!(matches!(sum.validation, Validation::Available));
    assert_eq!(sum.items[0].path, vec![confy_core::model::node::Seg::Key("port".into())]);
}

#[test]
fn a_broken_pattern_says_unavailable_never_no_problems() {
    let s = serde_json::json!({"properties":{"a":{"type":"string","pattern":"("}}});
    let c = project(&s, Some(&parse_toml("a = \"x\"\n")), &host());
    assert!(
        matches!(
            summary(&c.root, &c.state).validation,
            Validation::Unavailable { .. }
        ),
        "C6/D8"
    );
}
