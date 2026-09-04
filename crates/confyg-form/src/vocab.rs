//! The nine-member **Presentation vocabulary**, carried by the `x-confyg` **Annotation**.
//!
//! Presentation §2 is the authoritative table and this module matches it member for member.
//! Every member is optional, so `Presentation::default()` is the empty override.
//!
//! An unknown or unparseable member is a **Notice**, never an error, and never costs the known
//! members (ADR 0005).

use std::collections::BTreeMap;

use serde_json::Value;

use crate::ir::Widget;
use crate::notice::Notice;

/// The annotation key. The same shape appears as a v0.2 profile `nodes` entry.
pub const ANNOTATION: &str = "x-confyg";

/// A **Profile hint** at the Schema root only; not a vocabulary member.
const PROFILE: &str = "profile";

const MEMBERS: [&str; 9] = [
    "affordance",
    "order",
    "unit",
    "collapsed",
    "demoted",
    "label",
    "help",
    "labelFrom",
    "optionLabels",
];

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Presentation {
    pub affordance: Option<Widget>,
    pub order: Option<i64>,
    pub unit: Option<String>,
    pub collapsed: Option<bool>,
    pub demoted: Option<bool>,
    pub label: Option<String>,
    pub help: Option<String>,
    pub label_from: Option<String>,
    pub option_labels: Option<BTreeMap<String, String>>,
}

/// Parse a **Widget** name as written in presentation §4's table (`filterable-menu`) or as the IR
/// serializes it (`filterableMenu`).
pub fn widget_by_name(name: &str) -> Option<Widget> {
    let normalized: String = name
        .chars()
        .filter(|c| *c != '-' && *c != '_')
        .flat_map(char::to_lowercase)
        .collect();
    Some(match normalized.as_str() {
        "text" => Widget::Text,
        "raw" | "rawtext" => Widget::RawText,
        "displayonly" => Widget::DisplayOnly,
        "radio" => Widget::Radio,
        "menu" => Widget::Menu,
        "filterablemenu" => Widget::FilterableMenu,
        "checkboxset" => Widget::CheckboxSet,
        "tristate" => Widget::Tristate,
        "stepper" => Widget::Stepper,
        "slider" => Widget::Slider,
        "textarea" => Widget::Textarea,
        "masked" => Widget::Masked,
        _ => return None,
    })
}

/// Read the `x-confyg` annotation off one subschema.
pub fn read(schema: &Value) -> (Presentation, Vec<Notice>) {
    let mut p = Presentation::default();
    let mut notices = Vec::new();

    let Some(annotation) = schema.get(ANNOTATION).and_then(Value::as_object) else {
        return (p, notices);
    };

    for (key, value) in annotation {
        if key == PROFILE {
            continue; // the Profile hint is read by `profile_hint`, and must not warn.
        }
        if !MEMBERS.contains(&key.as_str()) {
            notices.push(Notice::new(
                "form.vocab.unknown-member",
                format!("`{ANNOTATION}.{key}` is not a Presentation vocabulary member; ignored"),
            ));
            continue;
        }
        // A member present but of the wrong type is also a Notice: taking the known members is
        // worth more than rejecting the annotation wholesale.
        let mut wrong_type = || {
            notices.push(Notice::new(
                "form.vocab.wrong-type",
                format!("`{ANNOTATION}.{key}` has an unusable value; ignored"),
            ));
        };
        match key.as_str() {
            "affordance" => match value.as_str().and_then(widget_by_name) {
                Some(w) => p.affordance = Some(w),
                None => wrong_type(),
            },
            "order" => match value.as_i64() {
                Some(n) => p.order = Some(n),
                None => wrong_type(),
            },
            "unit" => match value.as_str() {
                Some(s) => p.unit = Some(s.to_owned()),
                None => wrong_type(),
            },
            "collapsed" => match value.as_bool() {
                Some(b) => p.collapsed = Some(b),
                None => wrong_type(),
            },
            "demoted" => match value.as_bool() {
                Some(b) => p.demoted = Some(b),
                None => wrong_type(),
            },
            "label" => match value.as_str() {
                Some(s) => p.label = Some(s.to_owned()),
                None => wrong_type(),
            },
            "help" => match value.as_str() {
                Some(s) => p.help = Some(s.to_owned()),
                None => wrong_type(),
            },
            "labelFrom" => match value.as_str() {
                Some(s) => p.label_from = Some(s.to_owned()),
                None => wrong_type(),
            },
            "optionLabels" => match value.as_object() {
                Some(map) => {
                    p.option_labels = Some(
                        map.iter()
                            .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_owned())))
                            .collect(),
                    )
                }
                None => wrong_type(),
            },
            _ => unreachable!("MEMBERS and this match are the same list"),
        }
    }

    (p, notices)
}

/// The **Profile hint**: `x-confyg.profile` at the Schema root. v0.1 reads it and does nothing
/// with it — the sidecar is v0.2 (presentation §9) — but reading it here keeps the hint from
/// being reported as an unknown member.
pub fn profile_hint(root: &Value) -> Option<String> {
    root.get(ANNOTATION)?
        .get(PROFILE)?
        .as_str()
        .map(str::to_owned)
}
