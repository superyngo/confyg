use confyg_form::{ir::Widget, vocab};

#[test]
fn reads_all_nine_members() {
    let s = serde_json::json!({"x-confyg": {
        "affordance": "stepper", "order": 3, "unit": "MiB", "collapsed": true,
        "demoted": true, "label": "Cache size", "help": "per worker",
        "labelFrom": "name", "optionLabels": {"aes-256-gcm": "AES-256-GCM"}
    }});
    let (p, notices) = vocab::read(&s);
    assert_eq!(p.affordance, Some(Widget::Stepper));
    assert_eq!(p.order, Some(3));
    assert_eq!(p.unit.as_deref(), Some("MiB"));
    assert_eq!(p.collapsed, Some(true));
    assert_eq!(p.demoted, Some(true));
    assert_eq!(p.label.as_deref(), Some("Cache size"));
    assert_eq!(p.help.as_deref(), Some("per worker"));
    assert_eq!(p.label_from.as_deref(), Some("name"));
    assert_eq!(
        p.option_labels.as_ref().unwrap()["aes-256-gcm"],
        "AES-256-GCM"
    );
    assert!(notices.is_empty());
}

#[test]
fn an_unknown_member_is_a_notice_not_an_error() {
    let s = serde_json::json!({"x-confyg": {"hidden": true}});
    let (p, notices) = vocab::read(&s);
    assert!(p.affordance.is_none());
    assert_eq!(
        notices.len(),
        1,
        "there is no `hidden`; unknown keys are Notices (ADR 0005)"
    );
}

#[test]
fn an_unknown_member_never_costs_the_known_ones() {
    let s = serde_json::json!({"x-confyg": {"hidden": true, "affordance": "slider", "order": 2}});
    let (p, notices) = vocab::read(&s);
    assert_eq!(p.affordance, Some(Widget::Slider));
    assert_eq!(p.order, Some(2));
    assert_eq!(notices.len(), 1);
}

#[test]
fn an_unparseable_member_is_a_notice_too() {
    let s = serde_json::json!({"x-confyg": {"affordance": "hologram", "unit": "MiB"}});
    let (p, notices) = vocab::read(&s);
    assert!(p.affordance.is_none());
    assert_eq!(p.unit.as_deref(), Some("MiB"));
    assert_eq!(notices.len(), 1);
}

#[test]
fn profile_hint_lives_at_the_root_only() {
    let root = serde_json::json!({"x-confyg": {"profile": "./app.confyg.toml"}});
    assert_eq!(
        vocab::profile_hint(&root).as_deref(),
        Some("./app.confyg.toml")
    );
    let (p, notices) = vocab::read(&root);
    assert!(
        p.affordance.is_none() && notices.is_empty(),
        "profile is not a vocabulary member and must not warn"
    );
}

#[test]
fn no_annotation_is_the_empty_override() {
    let (p, notices) = vocab::read(&serde_json::json!({"type": "string"}));
    assert!(notices.is_empty());
    assert_eq!(p.order, None);
    assert!(vocab::profile_hint(&serde_json::json!({})).is_none());
}
