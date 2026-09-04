//! D7 and D1: the two hazards whose failure mode is misplaced text rather than an error.

use confy_core::model::any_doc::AnyDocument;
use confy_core::model::document::DocFormat;
use confy_core::model::node::Path;
use confyg_session::ordinal::{child_ordinal, schema_slot};

fn parse(src: &str, fmt: DocFormat) -> AnyDocument {
    AnyDocument::from_str_as(src, fmt).expect("parse")
}

fn parse_toml(src: &str) -> AnyDocument {
    parse(src, DocFormat::Toml)
}

fn root() -> Path {
    Vec::new()
}

fn order(keys: &[&str]) -> Vec<String> {
    keys.iter().map(|k| (*k).to_owned()).collect()
}
#[test]
fn projection_index_is_not_the_target_index() {
    // Consecutive `#` lines merge into ONE Comment node upstream (`model/cst_project.rs`: "a
    // blank line splits"), so three comment *lines* are one slot and projection 0 is ordinal 1.
    let doc = parse_toml("# a\n# b\n# c\nx = 1\ny = 2\n");
    assert_eq!(
        child_ordinal(&doc, &root(), 0),
        1,
        "Target.index counts comments (D7)"
    );
    assert_eq!(child_ordinal(&doc, &root(), 1), 2);

    // Blank-line-separated comments are three blocks, hence three slots.
    let split = parse_toml("# a\n\n# b\n\n# c\nx = 1\n");
    assert_eq!(child_ordinal(&split, &root(), 0), 3);
}

#[test]
fn a_comment_between_two_entries_shifts_the_later_one() {
    let doc = parse_toml("x = 1\n# between\ny = 2\n");
    assert_eq!(child_ordinal(&doc, &root(), 0), 0);
    assert_eq!(child_ordinal(&doc, &root(), 1), 2);
    assert_eq!(child_ordinal(&doc, &root(), 2), 3, "past the end appends");
}

#[test]
fn a_missing_key_lands_at_its_schema_position_among_present_siblings() {
    let doc = parse_toml("host = \"h\"\nca_cert = \"c\"\n");
    assert_eq!(
        schema_slot(&doc, &root(), "port", &order(&["host", "port", "ca_cert"])),
        1,
        "never appended blindly (design §8)"
    );
}

#[test]
fn a_key_the_schema_does_not_name_is_appended() {
    let doc = parse_toml("host = \"h\"\nca_cert = \"c\"\n");
    assert_eq!(
        schema_slot(&doc, &root(), "zzz", &order(&["host", "ca_cert"])),
        2
    );
}

#[test]
fn a_scalar_never_lands_after_a_sub_table_in_toml() {
    let doc = parse_toml("host = \"h\"\n[tls]\non = true\n");
    assert_eq!(
        schema_slot(&doc, &root(), "port", &order(&["host", "tls", "port"])),
        1,
        "legality wins over Schema order at the root, which does not clamp (D1)"
    );
}

#[test]
fn the_toml_partition_does_not_apply_to_json_or_yaml() {
    let json = parse(
        "{\n  \"host\": \"h\",\n  \"tls\": { \"on\": true }\n}\n",
        DocFormat::Json,
    );
    assert_eq!(
        schema_slot(&json, &root(), "port", &order(&["host", "tls", "port"])),
        2
    );
    let yaml = parse("host: h\ntls:\n  on: true\n", DocFormat::Yaml);
    assert_eq!(
        schema_slot(&yaml, &root(), "port", &order(&["host", "tls", "port"])),
        2
    );
}

#[test]
fn yaml_subtracts_the_root_comment_prefix_json_does_not() {
    let yaml = parse("# a\n# b\nx: 1\ny: 2\n", DocFormat::Yaml);
    assert_eq!(
        child_ordinal(&yaml, &root(), 0),
        0,
        "YAML root indices are container indices, not projection indices"
    );
    assert_eq!(child_ordinal(&yaml, &root(), 1), 1);

    let json = parse("{\n  // a\n  \"x\": 1,\n  \"y\": 2\n}\n", DocFormat::Json);
    assert_eq!(child_ordinal(&json, &root(), 0), 1);
    assert_eq!(child_ordinal(&json, &root(), 1), 2);
}

#[test]
fn a_nested_parent_counts_its_own_comments_in_every_format() {
    for (src, fmt) in [
        ("[tls]\n# lead\non = true\n", DocFormat::Toml),
        (
            "{ \"tls\": { /* lead */ \"on\": true } }\n",
            DocFormat::Json,
        ),
        ("tls:\n  # lead\n  on: true\n", DocFormat::Yaml),
    ] {
        let doc = parse(src, fmt);
        let tls = vec![confy_core::model::node::Seg::Key("tls".into())];
        assert_eq!(
            child_ordinal(&doc, &tls, 0),
            1,
            "{fmt:?}: a non-root parent never subtracts a prefix"
        );
    }
}
