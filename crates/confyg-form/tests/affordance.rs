use confyg_form::{affordance::*, facts::facts, ir::Widget, vocab};

fn host() -> HostProfile {
    HostProfile {
        can_mask: true,
        can_slide: true,
        can_filter_options: false,
        density: Density::Desktop,
    }
}

#[test]
fn menu_family_thresholds_are_constants() {
    let mk = |n: usize| {
        let vals: Vec<_> = (0..n).map(|i| serde_json::json!(i)).collect();
        facts(&serde_json::json!({"enum": vals}))
    };
    assert_eq!(derive(&mk(4), false), Widget::Radio);
    assert_eq!(derive(&mk(5), false), Widget::Menu);
    assert_eq!(derive(&mk(12), false), Widget::Menu);
    assert_eq!(derive(&mk(13), false), Widget::FilterableMenu);
}

#[test]
fn precedence_puts_raw_fallback_above_the_override() {
    let f = facts(&serde_json::json!({"type": "integer", "minimum": 0, "maximum": 9}));
    let p = vocab::read(&serde_json::json!({"x-confyg": {"affordance": "text"}})).0;
    assert_eq!(resolve(&f, &p, true, &host()).0, Widget::RawText);
    assert_eq!(
        resolve(&f, &p, false, &host()).0,
        Widget::Text,
        "override beats derivation"
    );
    assert_eq!(
        resolve(&f, &Default::default(), false, &host()).0,
        Widget::Slider,
        "both bounds present derives a slider"
    );
}

#[test]
fn const_and_read_only_outrank_everything() {
    let f = facts(&serde_json::json!({"const": 7}));
    let p = vocab::read(&serde_json::json!({"x-confyg": {"affordance": "stepper"}})).0;
    assert_eq!(resolve(&f, &p, false, &host()).0, Widget::DisplayOnly);
    let ro = facts(&serde_json::json!({"type": "string", "readOnly": true}));
    assert_eq!(derive(&ro, false), Widget::DisplayOnly);
}

#[test]
fn clamp_substitutes_but_keeps_intended() {
    let f = facts(&serde_json::json!({"type": "string", "writeOnly": true}));
    let h = HostProfile {
        can_mask: false,
        ..host()
    };
    let (w, intended, notices) = resolve(&f, &Default::default(), false, &h);
    assert_eq!((w, intended), (Widget::Text, Widget::Masked));
    assert_eq!(
        notices.len(),
        1,
        "a writeOnly value shown unmasked must say so, never silently (design §7 A4)"
    );
}

#[test]
fn filterable_menu_clamps_to_menu_in_v0_1() {
    let vals: Vec<_> = (0..400).map(|i| serde_json::json!(i)).collect();
    let f = facts(&serde_json::json!({"enum": vals}));
    let (w, intended, _) = resolve(&f, &Default::default(), false, &host());
    assert_eq!((w, intended), (Widget::Menu, Widget::FilterableMenu));
}

#[test]
fn every_ladder_terminates_in_a_universal_control() {
    for w in ALL_WIDGETS {
        let terminal = *ladder(w).last().unwrap_or(&w);
        assert!(
            matches!(
                terminal,
                Widget::Text | Widget::Radio | Widget::Tristate | Widget::DisplayOnly | Widget::RawText
            ),
            "{w:?} has no chain to a universally available control (ADR 0004)"
        );
    }
}

#[test]
fn a_slider_without_a_slide_capable_host_becomes_a_stepper() {
    let f = facts(&serde_json::json!({"type": "integer", "minimum": 0, "maximum": 9}));
    let h = HostProfile {
        can_slide: false,
        ..host()
    };
    let (w, intended, _) = resolve(&f, &Default::default(), false, &h);
    assert_eq!((w, intended), (Widget::Stepper, Widget::Slider));
}
