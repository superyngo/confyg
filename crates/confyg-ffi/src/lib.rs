//! The WASM boundary. Three exports, and **no logic**: everything here is serialization, so a
//! native host (the TUI at v0.3) can link `confyg-session` directly and skip this crate.
//!
//! `dispatch` is a `Request` in and a `SetterSnapshot` out. `check` exists so a live `pattern`
//! check crosses the boundary instead of being reimplemented host-side against a different
//! regex flavour — the form's warnings and the validator's Violations can then never disagree.
//! `search` is here for the same reason in a different key: **Form search** is the compiler's
//! (presentation §5.3), so the Web and TUI cannot rank the same query differently.

use confy_core::model::node::Path;
use confyg_session::session::{Request, Session, SetterSnapshot};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct Handle {
    session: Session,
}

impl Default for Handle {
    fn default() -> Self {
        Handle::new()
    }
}

#[wasm_bindgen]
impl Handle {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Handle {
        Handle {
            session: Session::new(),
        }
    }
}

/// A `Request` JSON in, a `SetterSnapshot` JSON out. A malformed request is reported in the
/// same envelope rather than trapping: a panic across the WASM boundary loses the session.
#[wasm_bindgen]
pub fn dispatch(state: &mut Handle, request_json: &str) -> String {
    match serde_json::from_str::<Request>(request_json) {
        Ok(req) => render(&state.session.dispatch(req)),
        Err(e) => error_envelope(&format!("malformed request: {e}")),
    }
}

/// The validator's own Violations for a buffer that is not yet committed. `path_json` is a
/// `Path` (`["a", {"index": 0}]`-shaped); the literal is raw text, exactly as typed.
#[wasm_bindgen]
pub fn check(state: &Handle, path_json: &str, literal: &str) -> String {
    match serde_json::from_str::<Path>(path_json) {
        Ok(path) => serde_json::to_string(&state.session.check(&path, literal))
            .unwrap_or_else(|_| "[]".to_owned()),
        Err(e) => error_envelope(&format!("malformed path: {e}")),
    }
}

/// **Form search** hits for `query`, best first — titles, descriptions and **Paths**, scored
/// by the compiler. The query is raw text, so it needs no JSON quoting; an empty one comes
/// back as an empty list rather than as the whole tree.
#[wasm_bindgen]
pub fn search(state: &Handle, query: &str) -> String {
    serde_json::to_string(&state.session.search(query)).unwrap_or_else(|_| "[]".to_owned())
}

fn render(snap: &SetterSnapshot) -> String {
    serde_json::to_string(snap).unwrap_or_else(|e| error_envelope(&e.to_string()))
}

fn error_envelope(message: &str) -> String {
    serde_json::json!({ "error": message }).to_string()
}
