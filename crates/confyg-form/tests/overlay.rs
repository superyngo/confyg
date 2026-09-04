use confyg_form::affordance::{Density, HostProfile};
use confyg_form::compile::{key_of, project};
use confyg_form::ir::{FormNode, Locked, Occupancy, Presence};

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

fn parse(src: &str, fmt: DocFormat) -> AnyDocument {
    AnyDocument::from_str_as(src, fmt).expect("parse")
}

fn parse_toml(src: &str) -> AnyDocument {
    parse(src, DocFormat::Toml)
}

fn parse_yaml(src: &str) -> AnyDocument {
    parse(src, DocFormat::Yaml)
}

fn child<'a>(root: &'a FormNode, key: &str) -> &'a FormNode {
    let FormNode::Group { children, .. } = root else {
        panic!("expected a Group")
    };
    children
        .iter()
        .find(|c| key_of(c) == Some(key))
        .unwrap_or_else(|| panic!("no child {key}"))
}

fn presence<'a>(root: &'a FormNode, key: &str) -> &'a Presence {
    match child(root, key) {
        FormNode::Field { presence, .. } => presence,
        other => panic!("expected a Field, got {other:?}"),
    }
}

fn occ(root: &FormNode, key: &str) -> Occupancy {
    match child(root, key) {
        FormNode::Repeat { occupancy, .. } | FormNode::Group { occupancy, .. } => *occupancy,
        other => panic!("expected a container, got {other:?}"),
    }
}

fn violations<'a>(root: &'a FormNode, key: &str) -> &'a [Violation] {
    match child(root, key) {
        FormNode::Group { meta, .. } | FormNode::Repeat { meta, .. } => &meta.violations,
        FormNode::Field { meta, .. } => &meta.node.violations,
        other => panic!("no meta on {other:?}"),
    }
}

fn locked(root: &FormNode, key: &str) -> Option<Locked> {
    match child(root, key) {
        FormNode::Group { meta, .. } | FormNode::Repeat { meta, .. } => meta.locked,
        FormNode::Field { meta, .. } => meta.node.locked,
        other => panic!("no meta on {other:?}"),
    }
}

fn root_violations(root: &FormNode) -> &[Violation] {
    match root {
        FormNode::Group { meta, .. } => &meta.violations,
        other => panic!("expected a Group root, got {other:?}"),
    }
}

#[test]
fn three_presence_states_and_the_default_stays_unwritten() {
    let s = serde_json::json!({"properties":{
        "host":{"type":"string"},
        "level":{"type":"string","enum":["info","debug"],"default":"info"},
        "port":{"type":"integer"}}});
    let doc = parse_toml("host = \"a\"\nport = \"nope\"\n");
    let c = project(&s, Some(&doc), &host());
    assert!(matches!(presence(&c.root, "host"), Presence::Set { .. }));
    assert!(
        matches!(
            presence(&c.root, "level"),
            Presence::Absent {
                default: Some(_),
                ..
            }
        ),
        "an unwritten default is Absent, not Set"
    );
    assert!(
        matches!(presence(&c.root, "port"), Presence::Invalid { literal, .. } if literal.contains("nope")),
        "a type mismatch is a Violation, and the literal stays as authored"
    );
}

#[test]
fn empty_and_absent_collections_differ() {
    let s =
        serde_json::json!({"properties":{"servers":{"type":"array","items":{"type":"string"}}}});
    assert_eq!(
        occ(
            &project(&s, Some(&parse_toml("servers = []\n")), &host()).root,
            "servers"
        ),
        Occupancy::Empty
    );
    assert_eq!(
        occ(&project(&s, Some(&parse_toml("")), &host()).root, "servers"),
        Occupancy::Absent
    );
    assert_eq!(
        occ(
            &project(&s, Some(&parse_toml("servers = [\"a\"]\n")), &host()).root,
            "servers"
        ),
        Occupancy::Populated
    );
}

#[test]
fn a_container_keyword_violation_attaches_to_the_container() {
    let s = serde_json::json!({"properties":{
        "servers":{"type":"array","items":{"type":"string"},"minItems":2}}});
    let c = project(&s, Some(&parse_toml("servers = [\"a\"]\n")), &host());
    assert_eq!(
        violations(&c.root, "servers").len(),
        1,
        "minItems reports the container's path (upstream.md Schema validation)"
    );
}

#[test]
fn a_required_failure_attaches_to_the_parent_object() {
    let s = serde_json::json!({"required":["host"],"properties":{"host":{"type":"string"}}});
    let c = project(&s, Some(&parse_toml("")), &host());
    assert_eq!(
        root_violations(&c.root).len(),
        1,
        "a required pointer names the parent, and PointerMap resolves it there"
    );
}

#[test]
fn a_yaml_alias_is_locked() {
    let s = serde_json::json!({"properties":{"b":{"type":"object",
        "properties":{"host":{"type":"string"}}}}});
    let doc = parse_yaml("a: &x\n  host: h\nb: *x\n");
    assert!(
        locked(&project(&s, Some(&doc), &host()).root, "b").is_some(),
        "an alias renders its resolved value with no write affordance (D/B21)"
    );
}

#[test]
fn repeat_items_are_projected_one_per_element() {
    let s = serde_json::json!({"properties":{"servers":{"type":"array",
        "items":{"type":"object","properties":{"host":{"type":"string"}}}}}});
    let doc = parse_toml("[[servers]]\nhost = \"a\"\n\n[[servers]]\nhost = \"b\"\n");
    let c = project(&s, Some(&doc), &host());
    match child(&c.root, "servers") {
        FormNode::Repeat { items, .. } => {
            assert_eq!(items.len(), 2);
            assert!(matches!(
                presence(&items[1], "host"),
                Presence::Set { literal } if literal.contains('b')
            ));
        }
        other => panic!("expected a Repeat, got {other:?}"),
    }
}

#[test]
fn an_uncompilable_schema_still_overlays_the_document() {
    let s = serde_json::json!({"properties":{"a":{"type":"string","pattern":"("}}});
    let c = project(&s, Some(&parse_toml("a = \"x\"\n")), &host());
    assert!(matches!(presence(&c.root, "a"), Presence::Set { .. }));
    assert!(c.state.validatable.is_err());
}

#[test]
fn the_same_document_in_three_formats_projects_the_same_shape() {
    let s = serde_json::json!({"properties":{
        "host":{"type":"string"},
        "tls":{"type":"object","properties":{"on":{"type":"boolean"}}}}});
    let shape = |doc: &AnyDocument| {
        let c = project(&s, Some(doc), &host());
        (
            matches!(presence(&c.root, "host"), Presence::Set { .. }),
            occ(&c.root, "tls"),
        )
    };
    let toml = shape(&parse_toml("host = \"a\"\n[tls]\non = true\n"));
    let json = shape(&parse(
        "{\"host\": \"a\", \"tls\": {\"on\": true}}",
        DocFormat::Json,
    ));
    let yaml = shape(&parse_yaml("host: a\ntls:\n  on: true\n"));
    assert_eq!(toml, json);
    assert_eq!(toml, yaml);
    assert_eq!(toml, (true, Occupancy::Populated));
}
