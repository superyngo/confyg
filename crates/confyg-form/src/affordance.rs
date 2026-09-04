//! **Widget** derivation, the menu family, the **Degradation ladder**, and the `HostProfile`
//! clamp. Presentation §3 and §4.
//!
//! `resolve` is derive → override → clamp. The clamp walks the ladder until the host can render a
//! rung; it never re-resolves, and it keeps the pre-clamp choice as `intended` so the form can
//! explain the substitution (presentation §3, §6).

use crate::facts::SchemaFacts;
use crate::ir::Widget;
use crate::notice::Notice;
use crate::vocab::Presentation;

/// The menu-family thresholds. Constants, not settings (ADR 0004 decision 3): a node wanting a
/// different outcome writes `x-confyg.affordance`.
pub const RADIO_MAX_OPTIONS: usize = 4;
pub const MENU_MAX_OPTIONS: usize = 12;

/// Every member of the closed Widget vocabulary, for exhaustive tests and host registries.
pub const ALL_WIDGETS: [Widget; 12] = [
    Widget::Text,
    Widget::RawText,
    Widget::DisplayOnly,
    Widget::Radio,
    Widget::Menu,
    Widget::FilterableMenu,
    Widget::CheckboxSet,
    Widget::Tristate,
    Widget::Stepper,
    Widget::Slider,
    Widget::Textarea,
    Widget::Masked,
];

/// Row height / hit-target scale. Only `Desktop` is wired in v0.1; the enum exists so the clamp
/// signature does not change when presentation §10's mapping is answered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Density {
    Desktop,
    Phone,
    Touch,
}

/// What the host declared it can render. Pure data, so a clamped form is an assertion rather
/// than a manual check (presentation §4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HostProfile {
    pub can_mask: bool,
    pub can_slide: bool,
    pub can_filter_options: bool,
    pub density: Density,
}

/// Presentation §4's ladder table. Every chain terminates in a control every host has.
pub fn ladder(w: Widget) -> &'static [Widget] {
    match w {
        Widget::FilterableMenu => &[Widget::Menu, Widget::Radio],
        Widget::Menu => &[Widget::Radio],
        Widget::CheckboxSet => &[Widget::Radio],
        Widget::Slider => &[Widget::Stepper, Widget::Text],
        Widget::Stepper => &[Widget::Text],
        Widget::Masked => &[Widget::Text],
        Widget::Textarea => &[Widget::Text],
        // Radio, Tristate, Text, RawText and DisplayOnly are the terminals themselves.
        Widget::Radio | Widget::Tristate | Widget::Text | Widget::RawText | Widget::DisplayOnly => {
            &[]
        }
    }
}

fn menu_family(option_count: usize) -> Widget {
    if option_count <= RADIO_MAX_OPTIONS {
        Widget::Radio
    } else if option_count <= MENU_MAX_OPTIONS {
        Widget::Menu
    } else {
        Widget::FilterableMenu
    }
}

/// Presentation §3's precedence, steps 1, 2, 4, 5, 6 — everything but the override, which is
/// `resolve`'s job.
///
/// `raw` is the **Raw literal fallback**: the literal in the Document cannot be edited as its
/// declared type, so the only honest control is raw text (design §7 A23).
pub fn derive(f: &SchemaFacts, raw: bool) -> Widget {
    if f.const_value.is_some() || f.read_only {
        return Widget::DisplayOnly;
    }
    if raw {
        return Widget::RawText;
    }
    if let Some(options) = &f.enum_values {
        return menu_family(options.len());
    }
    if f.write_only {
        return Widget::Masked;
    }
    // `format` → specialized control is v0.2+ (design §7 A12-A19); until then those nodes fall
    // through to the primitive control, which is exactly their ladder terminal anyway.
    match f.ty.as_ref().and_then(|t| t.sole()) {
        Some("boolean") => Widget::Tristate,
        Some("integer") | Some("number") => {
            let has_min = f.bounds.min.is_some() || f.bounds.exclusive_min.is_some();
            let has_max = f.bounds.max.is_some() || f.bounds.exclusive_max.is_some();
            if has_min && has_max {
                Widget::Slider
            } else if has_min || has_max || f.multiple_of.is_some() {
                Widget::Stepper
            } else {
                Widget::Text
            }
        }
        _ => Widget::Text,
    }
}

fn can_render(w: Widget, host: &HostProfile) -> bool {
    match w {
        Widget::FilterableMenu => host.can_filter_options,
        Widget::Slider => host.can_slide,
        Widget::Masked => host.can_mask,
        _ => true,
    }
}

/// derive → override → clamp, returning `(widget, intended, notices)`.
pub fn resolve(
    f: &SchemaFacts,
    p: &Presentation,
    raw: bool,
    host: &HostProfile,
) -> (Widget, Widget, Vec<Notice>) {
    let derived = derive(f, raw);
    // Steps 1 and 2 outrank the override: a `const` is not editable and a raw literal is not
    // interpretable, whatever the annotation asks for.
    let intended = match derived {
        Widget::DisplayOnly | Widget::RawText => derived,
        _ => p.affordance.unwrap_or(derived),
    };

    let mut notices = Vec::new();
    let mut widget = intended;
    if !can_render(widget, host) {
        for rung in ladder(intended) {
            widget = *rung;
            if can_render(widget, host) {
                break;
            }
        }
        notices.push(degrade_notice(intended, widget));
    }
    (widget, intended, notices)
}

fn degrade_notice(intended: Widget, shown: Widget) -> Notice {
    let name = |w: Widget| {
        serde_json::to_value(w)
            .ok()
            .and_then(|v| v.as_str().map(str::to_owned))
            .unwrap_or_default()
    };
    // `masked` degrading to plain text is a security surprise if it happens silently, so it is
    // stated in the message rather than left to the generic wording (design §7 A4).
    let code = if intended == Widget::Masked {
        "form.degrade.masked"
    } else {
        "form.degrade.generic"
    };
    Notice::new(
        code,
        format!(
            "This environment cannot render {}; showing {} instead.",
            name(intended),
            name(shown)
        ),
    )
}
