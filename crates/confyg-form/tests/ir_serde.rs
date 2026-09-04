use confyg_form::ir::*;

#[test]
fn absent_field_serializes_with_its_default_and_intended_widget() {
    let node = FormNode::Field {
        path: vec![],
        widget: Widget::Menu,
        intended: Widget::FilterableMenu,
        presence: Presence::Absent {
            default: Some(serde_json::json!("info")),
            remarked: None,
        },
        meta: FieldMeta::default(),
    };
    let v = serde_json::to_value(&node).unwrap();
    assert_eq!(v["kind"], "field");
    assert_eq!(v["widget"], "menu");
    assert_eq!(v["intended"], "filterableMenu");
    assert_eq!(v["presence"]["kind"], "absent");
    assert_eq!(v["presence"]["default"], "info");
}

#[test]
fn occupancy_distinguishes_empty_from_absent() {
    assert_ne!(
        serde_json::to_value(Occupancy::Empty).unwrap(),
        serde_json::to_value(Occupancy::Absent).unwrap(),
        "servers = [] and a missing servers key are different facts (ADR 0003)"
    );
}
