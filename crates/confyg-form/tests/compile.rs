use confyg_form::affordance::{Density, HostProfile};
use confyg_form::compile::compile;
use confyg_form::ir::{FormNode, Widget};

fn host() -> HostProfile {
    HostProfile {
        can_mask: true,
        can_slide: true,
        can_filter_options: false,
        density: Density::Desktop,
    }
}

fn child_keys(root: &FormNode) -> Vec<String> {
    let FormNode::Group { children, .. } = root else {
        panic!("expected a Group, got {root:?}")
    };
    children
        .iter()
        .map(|c| match confyg_form::compile::key_of(c) {
            Some(k) => k.to_owned(),
            None => "<root>".to_owned(),
        })
        .collect()
}

fn has_cyclic(node: &FormNode) -> bool {
    match node {
        FormNode::Cyclic { .. } => true,
        FormNode::Group { children, .. } => children.iter().any(has_cyclic),
        FormNode::Repeat { items, .. } => items.iter().any(has_cyclic),
        _ => false,
    }
}

#[test]
fn properties_become_a_group_in_schema_order_with_no_required_hoisting() {
    let s: serde_json::Value = serde_json::from_str(
        r#"{"type":"object","required":["port"],
            "properties":{"host":{"type":"string"},"port":{"type":"integer"}}}"#,
    )
    .unwrap();
    let c = compile(&s, &host());
    let keys = child_keys(&c.root);
    assert_eq!(
        keys,
        ["host", "port"],
        "the Schema author's order wins (design §4 step 5)"
    );
}

#[test]
fn x_confyg_order_overrides_and_demoted_sinks() {
    let s: serde_json::Value = serde_json::from_str(
        r#"{"properties":{"a":{"x-confyg":{"order":2}},"b":{"x-confyg":{"order":1}},
                          "c":{"x-confyg":{"demoted":true}}}}"#,
    )
    .unwrap();
    assert_eq!(child_keys(&compile(&s, &host()).root), ["b", "a", "c"]);
}

#[test]
fn a_scalar_array_is_a_repeat_of_fields_not_a_field() {
    let s = serde_json::json!({"type":"array","items":{"type":"string"},
                               "minItems":1,"maxItems":3});
    match compile(&s, &host()).root {
        FormNode::Repeat {
            bounds,
            item_template,
            ..
        } => {
            assert_eq!((bounds.min, bounds.max), (Some(1), Some(3)));
            assert_eq!(item_template.0, "#/items");
        }
        other => panic!("ADR 0003: a scalar array is a Repeat group, got {other:?}"),
    }
}

#[test]
fn ref_and_all_of_are_resolved_and_a_cycle_terminates() {
    let s: serde_json::Value = serde_json::from_str(
        r##"{"$defs":{"n":{"type":"object","properties":{"child":{"$ref":"#/$defs/n"}}}},
            "$ref":"#/$defs/n"}"##,
    )
    .unwrap();
    let c = compile(&s, &host()); // must return, not recurse forever
    assert!(
        has_cyclic(&c.root),
        "a self-referential $ref compiles to Cyclic"
    );
}

#[test]
fn all_of_members_merge_left_to_right() {
    let s = serde_json::json!({"properties":{"a":{
        "allOf":[{"type":"integer","minimum":0},{"maximum":9}]}}});
    let c = compile(&s, &host());
    let FormNode::Group { children, .. } = &c.root else {
        panic!()
    };
    match &children[0] {
        FormNode::Field { widget, .. } => assert_eq!(
            *widget,
            Widget::Slider,
            "both bounds arrive through the merge (design §4 step 2)"
        ),
        other => panic!("expected a Field, got {other:?}"),
    }
}

#[test]
fn a_v0_1_excluded_construct_is_unknown_with_a_notice() {
    let s = serde_json::json!({"properties":{
        "t":{"prefixItems":[{"type":"string"}]},
        "v":{"oneOf":[{"type":"object"},{"type":"string"}]}}});
    let c = compile(&s, &host());
    let FormNode::Group { children, .. } = &c.root else {
        panic!()
    };
    assert!(children
        .iter()
        .all(|c| matches!(c, FormNode::Unknown { .. })));
    assert_eq!(c.notices.len(), 2, "each names the tier that implements it");
}

#[test]
fn an_uncompilable_pattern_still_projects_a_complete_form() {
    let s = serde_json::json!({"properties":{"a":{"type":"string","pattern":"("}}});
    let c = compile(&s, &host());
    assert_eq!(child_keys(&c.root), ["a"]);
    assert!(
        c.state.validatable.is_err(),
        "D8: the document loses validation, not its form"
    );
}

#[test]
fn a_menu_field_carries_its_choices_already_labelled() {
    let s: serde_json::Value = serde_json::from_str(
        r#"{"type":"object","properties":{"cipher":{"type":"string",
            "enum":["aes-256-gcm","chacha20"],
            "x-confyg":{"optionLabels":{"aes-256-gcm":"AES-256-GCM"}}}}}"#,
    )
    .unwrap();
    let c = compile(&s, &host());
    let FormNode::Group { children, .. } = &c.root else {
        panic!("expected a Group")
    };
    let FormNode::Field { widget, meta, .. } = &children[0] else {
        panic!("expected a Field")
    };
    assert_eq!(*widget, Widget::Radio, "two choices is under the 4 floor");
    let labels: Vec<&str> = meta.options.iter().map(|o| o.label.as_str()).collect();
    assert_eq!(
        labels,
        ["AES-256-GCM", "chacha20"],
        "the label comes from the Annotation; an unlabelled value reads as authored"
    );
}

#[test]
fn a_field_with_no_enum_offers_no_choices() {
    let s: serde_json::Value =
        serde_json::from_str(r#"{"type":"object","properties":{"host":{"type":"string"}}}"#)
            .unwrap();
    let c = compile(&s, &host());
    let FormNode::Group { children, .. } = &c.root else {
        panic!("expected a Group")
    };
    let FormNode::Field { meta, .. } = &children[0] else {
        panic!("expected a Field")
    };
    assert!(meta.options.is_empty());
}
