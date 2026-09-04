//! `Constraint` extraction and its guidance text.
//!
//! The renderable subset of the Schema's constraints, and nothing else: this is guidance the
//! form shows beside a control, never a gate on a write (**Soft constraint**).

use crate::facts::SchemaFacts;
use crate::ir::Constraint;

/// Everything the form can usefully say about a value before it is written.
pub fn extract(f: &SchemaFacts) -> Vec<Constraint> {
    let mut out = Vec::new();
    if let Some(v) = f.bounds.min {
        out.push(Constraint::Minimum {
            value: v,
            exclusive: false,
        });
    }
    if let Some(v) = f.bounds.exclusive_min {
        out.push(Constraint::Minimum {
            value: v,
            exclusive: true,
        });
    }
    if let Some(v) = f.bounds.max {
        out.push(Constraint::Maximum {
            value: v,
            exclusive: false,
        });
    }
    if let Some(v) = f.bounds.exclusive_max {
        out.push(Constraint::Maximum {
            value: v,
            exclusive: true,
        });
    }
    if let Some(v) = f.multiple_of {
        out.push(Constraint::MultipleOf { value: v });
    }
    // `len` carries whichever pair the node declared; a Field only ever renders the string half,
    // since a collection's bounds live on the Repeat group's `bounds`.
    if let Some(v) = f.len.min {
        out.push(Constraint::MinLength { value: v });
    }
    if let Some(v) = f.len.max {
        out.push(Constraint::MaxLength { value: v });
    }
    if let Some(p) = &f.pattern {
        out.push(Constraint::Pattern { source: p.clone() });
    }
    if f.unique_items {
        out.push(Constraint::UniqueItems);
    }
    out
}

/// Developer-facing English for one constraint. A host with a **Lexicon** renders its own copy;
/// this exists so a bare host still says something true.
pub fn guidance(c: &Constraint) -> String {
    match c {
        Constraint::Minimum {
            value,
            exclusive: false,
        } => format!("at least {value}"),
        Constraint::Minimum {
            value,
            exclusive: true,
        } => format!("greater than {value}"),
        Constraint::Maximum {
            value,
            exclusive: false,
        } => format!("at most {value}"),
        Constraint::Maximum {
            value,
            exclusive: true,
        } => format!("less than {value}"),
        Constraint::MultipleOf { value } => format!("a multiple of {value}"),
        Constraint::MinLength { value } => format!("at least {value} characters"),
        Constraint::MaxLength { value } => format!("at most {value} characters"),
        Constraint::Pattern { source } => format!("matching {source}"),
        Constraint::UniqueItems => "entries must be unique".to_owned(),
    }
}
