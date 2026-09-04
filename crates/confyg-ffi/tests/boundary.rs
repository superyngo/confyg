//! The boundary is JSON in, JSON out, and it holds no logic of its own.

use confyg_ffi::{check, dispatch, Handle};
use serde_json::{json, Value};

fn handle_with(schema: Value, src: &str) -> Handle {
    let mut h = Handle::new();
    dispatch(
        &mut h,
        &json!({"command":{"open":{"text": src, "fmt":"Toml", "path":"a.toml"}}}).to_string(),
    );
    dispatch(
        &mut h,
        &json!({"command":{"loadSchema":{"source":{"Local":"a.schema.json"},
                                        "text": schema.to_string()}}})
        .to_string(),
    );
    h
}

#[test]
fn the_boundary_is_json_in_json_out() {
    let mut h = Handle::new();
    let out = dispatch(
        &mut h,
        &json!({"command":{"open":{"text":"host = \"a\"\n","fmt":"Toml","path":null}}}).to_string(),
    );
    let snap: Value = serde_json::from_str(&out).expect("snapshot JSON");
    assert!(snap["ir"].is_object() && snap["text"].is_string(), "{snap}");
    assert_eq!(snap["text"], "host = \"a\"\n");
}

#[test]
fn a_malformed_request_is_reported_never_a_panic() {
    let mut h = Handle::new();
    let out: Value = serde_json::from_str(&dispatch(&mut h, "{ not json")).expect("envelope");
    assert!(out["error"].is_string(), "{out}");
}

#[test]
fn an_intent_crosses_the_boundary_and_writes() {
    let mut h = handle_with(
        json!({"properties":{"port":{"type":"integer"}}}),
        "port = 80\n",
    );
    let out: Value = serde_json::from_str(&dispatch(
        &mut h,
        &json!({"intent":{"kind":"setValue","path":[{"Key":"port"}],"value":8080}}).to_string(),
    ))
    .expect("snapshot");
    assert_eq!(out["text"], "port = 8080\n", "{out}");
    assert_eq!(out["canUndo"], true);
}

#[test]
fn the_web_hosts_own_intent_json_is_accepted_verbatim() {
    // The exact strings `web/src/main.ts`'s `session` builds, byte for byte: a widget hands
    // over a JSON literal, the host parses it into a `value`, and `SetterIntent` is
    // internally tagged. A renamed tag here is a host that silently stops writing.
    let mut h = handle_with(
        json!({"properties":{"host":{"type":"string","default":"localhost"}}}),
        "host = \"a\"\n",
    );
    let set: Value = serde_json::from_str(&dispatch(
        &mut h,
        r#"{"intent":{"kind":"setValue","path":[{"Key":"host"}],"value":"b"}}"#,
    ))
    .expect("snapshot");
    assert_eq!(set["text"], "host = \"b\"\n", "{set}");
    let unset: Value = serde_json::from_str(&dispatch(
        &mut h,
        r#"{"intent":{"kind":"unset","path":[{"Key":"host"}]}}"#,
    ))
    .expect("snapshot");
    assert_eq!(unset["text"], "", "an Unset removes the key (ADR 0003)");
}

#[test]
fn live_checks_use_the_validators_own_engine() {
    // `fancy-regex` accepts a lookahead Rust's `regex` rejects; the form must agree with the
    // validator rather than with a second-guess implementation.
    let h = handle_with(
        json!({"properties":{"a":{"type":"string","pattern":"^(?!x)[a-z]+$"}}}),
        "a = \"ok\"\n",
    );
    let bad: Value =
        serde_json::from_str(&check(&h, "[{\"Key\":\"a\"}]", "xyz")).expect("violations");
    assert_eq!(
        bad.as_array().expect("array").len(),
        1,
        "form warnings and Violations never disagree (design §7): {bad}"
    );
    let good: Value =
        serde_json::from_str(&check(&h, "[{\"Key\":\"a\"}]", "yes")).expect("violations");
    assert!(good.as_array().expect("array").is_empty(), "{good}");
}
