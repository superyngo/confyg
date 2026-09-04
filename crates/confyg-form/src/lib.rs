//! `confyg-form` — the pure `Schema -> Form IR` compiler.
//!
//! No I/O and no state: every entry point is a function over
//! `(&serde_json::Value, ...)`. `confy_core::session` is never referenced.

pub mod facts;
pub mod ir;
