//! **Notice**: a fact the Schema did not produce, reported to the host without failing anything.
//!
//! Notices are how the **Soft constraint** extends to presentation input: an unknown `x-confyg`
//! member, an unparseable **Widget** name, a clamped affordance, and a v0.1-excluded Schema
//! construct are all Notices (ADR 0005). `code` is a **Lexicon** key; `message` is the
//! developer-facing English the host may show until a translation exists.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Notice {
    pub code: String,
    pub message: String,
}

impl Notice {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Notice {
            code: code.into(),
            message: message.into(),
        }
    }
}
