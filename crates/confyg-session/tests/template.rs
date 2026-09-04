//! Comment policy is derived (D4), and C3's real acceptance: the file confyg wrote resolves its
//! own Schema when it is reopened.

use confy_core::model::document::DocFormat;
use confy_core::schema::hints::detect_hint;
use confy_core::schema::SchemaSource;
use confyg_form::affordance::{Density, HostProfile};
use confyg_form::compile::compile;
use confyg_form::ir::FormNode;
use confyg_session::template::{comment_policy, generate, CommentPolicy, TemplateStrategy};
use serde_json::{json, Value};

fn host() -> HostProfile {
    HostProfile {
        can_mask: true,
        can_slide: true,
        can_filter_options: false,
        density: Density::Desktop,
    }
}

fn ir_of(schema: &Value) -> FormNode {
    compile(schema, &host()).root
}

fn titled_schema() -> Value {
    json!({"$id":"https://example.com/a.schema.json","required":["host"],
        "properties":{"host":{"type":"string","title":"Hostname","description":"DNS name"}}})
}

#[test]
fn comment_policy_is_derived_never_chosen() {
    assert!(matches!(
        comment_policy(Some("a.toml"), DocFormat::Toml, false),
        CommentPolicy::Allow
    ));
    assert!(matches!(
        comment_policy(Some("a.json"), DocFormat::Json, false),
        CommentPolicy::Deny
    ));
    assert!(matches!(
        comment_policy(Some("a.jsonc"), DocFormat::Json, false),
        CommentPolicy::Allow
    ));
    assert!(
        matches!(
            comment_policy(Some("a.json"), DocFormat::Json, true),
            CommentPolicy::Allow
        ),
        "a .json that arrived with comments has a consumer that tolerates them"
    );
    assert!(
        matches!(
            comment_policy(None, DocFormat::Json, false),
            CommentPolicy::Deny
        ),
        "a new file with no extension denies until the format dialog sets one"
    );
}

#[test]
fn a_template_carries_titles_as_comments_and_the_schema_hint() {
    let s = titled_schema();
    let out = generate(
        &s,
        &ir_of(&s),
        DocFormat::Toml,
        TemplateStrategy::RequiredOnly,
        CommentPolicy::Allow,
    );
    assert_eq!(
        out,
        "#:schema https://example.com/a.schema.json\n\n# Hostname\n# DNS name\nhost = \"\"\n"
    );
}

#[test]
fn a_strict_json_template_has_no_comments_at_all() {
    let s = titled_schema();
    let out = generate(
        &s,
        &ir_of(&s),
        DocFormat::Json,
        TemplateStrategy::RequiredOnly,
        CommentPolicy::Deny,
    );
    assert!(
        !out.lines().any(|l| l.trim_start().starts_with("//")),
        "D4: no comment line survives a strict-JSON template: {out:?}"
    );
    assert!(
        serde_json::from_str::<Value>(&out).is_ok(),
        "strict JSON must parse: {out:?}"
    );
}

#[test]
fn only_the_strategy_decides_what_is_written() {
    let s = json!({"required":["host"],
        "properties":{"host":{"type":"string"},"port":{"type":"integer","default":80},
                      "ca":{"type":"string"}}});
    let gen = |strategy| {
        generate(
            &s,
            &ir_of(&s),
            DocFormat::Toml,
            strategy,
            // The strategy is under test here, not the policy: `Deny` keeps the derived
            // `title` comments (which fall back to the key) out of the expected bytes.
            CommentPolicy::Deny,
        )
    };
    assert_eq!(gen(TemplateStrategy::RequiredOnly), "host = \"\"\n");
    assert_eq!(
        gen(TemplateStrategy::WithDefaults),
        "host = \"\"\nport = 80\n"
    );
    assert_eq!(
        gen(TemplateStrategy::Everything),
        "host = \"\"\nport = 80\nca = \"\"\n"
    );
}

/// C3: the reverse direction. A generated file must resolve its own Schema on reopen, in every
/// format — the hint's spelling is a means, this is the acceptance.
#[test]
fn a_generated_template_resolves_its_own_schema_on_reopen() {
    let s = titled_schema();
    for fmt in [DocFormat::Toml, DocFormat::Yaml, DocFormat::Json] {
        let policy = if fmt == DocFormat::Json {
            CommentPolicy::Deny
        } else {
            CommentPolicy::Allow
        };
        let out = generate(&s, &ir_of(&s), fmt, TemplateStrategy::RequiredOnly, policy);
        assert_eq!(
            detect_hint(&out, fmt),
            Some(SchemaSource::Url(
                "https://example.com/a.schema.json".to_owned()
            )),
            "{fmt:?} did not round-trip its hint: {out:?}"
        );
    }
}
