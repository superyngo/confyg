//! `confyg-form` — the pure `Schema -> Form IR` compiler.
//!
//! No I/O and no state: every entry point is a function over
//! `(&serde_json::Value, ...)`. `confy_core::session` is never referenced.

pub mod affordance;
pub mod compile;
pub mod constraint;
pub mod facts;
pub mod ir;
pub mod notice;
pub mod overlay;
pub mod search;
pub mod unknown;
pub mod vocab;
