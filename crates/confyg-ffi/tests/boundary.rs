//! The boundary is JSON in, JSON out, and it holds no logic of its own.

use confyg_ffi::{check, dispatch, search, Handle};
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

#[test]
fn the_web_hosts_own_collection_intent_json_is_accepted_verbatim() {
    // The exact strings `web/src/repeat.ts` hands to `main.ts` for a card's + and −. Both are
    // internally tagged on `kind` inside the externally tagged `Request`, and `index` counts
    // *entries* — `lower.rs` indexes the IR's `items`, never the Document's children, so a
    // host that sent a child index would delete a comment.
    let mut h = handle_with(
        json!({"properties":{"servers":{"type":"array","maxItems":3,
               "items":{"type":"object","required":["host"],
                        "properties":{"host":{"type":"string"}}}}}}),
        "[[servers]]\nhost = \"a.example\"\n",
    );
    let added: Value = serde_json::from_str(&dispatch(
        &mut h,
        r#"{"intent":{"kind":"addRepeatItem","path":[{"Key":"servers"}]}}"#,
    ))
    .expect("snapshot");
    assert_eq!(
        added["text"]
            .as_str()
            .expect("text")
            .matches("[[servers]]")
            .count(),
        2,
        "{added}"
    );
    let removed: Value = serde_json::from_str(&dispatch(
        &mut h,
        r#"{"intent":{"kind":"removeRepeatItem","path":[{"Key":"servers"}],"index":1}}"#,
    ))
    .expect("snapshot");
    assert_eq!(
        removed["text"], "[[servers]]\nhost = \"a.example\"\n",
        "removing the entry just added returns the original bytes: {removed}"
    );
}

#[test]
fn a_group_toggle_crosses_the_boundary_verbatim() {
    let mut h = handle_with(
        json!({"properties":{"tls":{"type":"object","required":["cert"],
               "properties":{"cert":{"type":"string"}}}}}),
        "host = \"a\"\n",
    );
    let on: Value = serde_json::from_str(&dispatch(
        &mut h,
        r#"{"intent":{"kind":"toggleGroup","path":[{"Key":"tls"}],"enable":true}}"#,
    ))
    .expect("snapshot");
    assert!(
        on["text"].as_str().expect("text").contains("[tls]"),
        "enabling an Absent Group writes its section: {on}"
    );
    let off: Value = serde_json::from_str(&dispatch(
        &mut h,
        r#"{"intent":{"kind":"toggleGroup","path":[{"Key":"tls"}],"enable":false}}"#,
    ))
    .expect("snapshot");
    assert_eq!(
        off["text"], "host = \"a\"\n",
        "a Group turned off is written by absence: {off}"
    );
}

#[test]
fn form_search_crosses_the_boundary_as_raw_text() {
    // `search` takes the query unquoted — it is not JSON — and hands back the compiler's own
    // ranking. A host that re-sorted this list would rank differently from the TUI (§5.3).
    let h = handle_with(
        json!({"properties":{"deadline":{"type":"integer","title":"Request timeout"}}}),
        "deadline = 30\n",
    );
    let hits: Value = serde_json::from_str(&search(&h, "timeout")).expect("hits");
    assert_eq!(hits[0]["path"], json!([{"Key":"deadline"}]), "{hits}");
    assert_eq!(hits[0]["title"], "Request timeout", "{hits}");
    let empty: Value = serde_json::from_str(&search(&h, "")).expect("hits");
    assert!(
        empty.as_array().expect("array").is_empty(),
        "an empty query is not every node: {empty}"
    );
}
