//! **Form search** lives in the compiler, not in a host (presentation §5.3): a result has to be
//! able to move the **Partition** to the section holding the hit, and two host-side
//! implementations would drift apart. These tests pin the three axes it matches over.

use confyg_form::affordance::{Density, HostProfile};
use confyg_form::compile::compile;
use confyg_form::search::{path_text, search};

fn host() -> HostProfile {
    HostProfile {
        can_mask: true,
        can_slide: true,
        can_filter_options: false,
        density: Density::Desktop,
    }
}

/// A Schema whose wording deliberately disagrees with its keys: `deadline` is titled
/// "Request timeout" and `cacheSize` is described in terms of memory. A key-only search
/// finds neither.
fn schema() -> serde_json::Value {
    serde_json::from_str(
        r#"{"type":"object","properties":{
             "server":{"type":"object","title":"Server","properties":{
               "deadline":{"type":"integer","title":"Request timeout"}}},
             "cacheSize":{"type":"integer",
                          "description":"How much memory to keep hot before evicting"},
             "tags":{"type":"array","items":{"type":"string"}}}}"#,
    )
    .unwrap()
}

fn paths(query: &str) -> Vec<String> {
    let c = compile(&schema(), &host());
    search(&c.root, query)
        .iter()
        .map(|hit| path_text(&hit.path))
        .collect()
}

#[test]
fn a_title_matches_when_the_key_never_says_it() {
    assert!(
        paths("timeout").contains(&"server.deadline".to_owned()),
        "the plan's own case: searching the word a user knows, not the key the author chose"
    );
}

#[test]
fn a_description_matches_when_the_key_never_says_it() {
    assert!(paths("memory").contains(&"cacheSize".to_owned()));
}

#[test]
fn a_path_still_matches() {
    // The third axis: a user who knows the key gets the node whatever its prose says.
    assert_eq!(paths("cachesize"), ["cacheSize"]);
}

#[test]
fn an_empty_query_finds_nothing() {
    // An Option filter treats an empty needle as "everything matches"; Form search is a
    // result list, and a list of every node is not a result (§3.1 keeps the two apart).
    assert!(paths("").is_empty());
}

#[test]
fn a_query_that_matches_nothing_is_empty_never_the_whole_tree() {
    assert!(paths("zzzznotathing").is_empty());
}

#[test]
fn a_hit_carries_the_title_a_host_would_show() {
    let c = compile(&schema(), &host());
    let hit = search(&c.root, "timeout")
        .into_iter()
        .find(|h| path_text(&h.path) == "server.deadline")
        .expect("a hit for the titled field");
    assert_eq!(hit.title, "Request timeout");
}

#[test]
fn a_stronger_match_outranks_a_weaker_one() {
    // Ordering is the whole reason a score crosses the boundary: the host renders the list
    // in the order the compiler returns it.
    let c = compile(&schema(), &host());
    let hits = search(&c.root, "server");
    assert_eq!(
        path_text(&hits.first().expect("at least one hit").path),
        "server",
        "the exactly-named Group beats its own children: {hits:?}"
    );
}

#[test]
fn path_text_matches_the_hosts_own_rendering() {
    // `web/src/types.ts` `pathText` renders `servers[0].host`; a divergent Rust rendering
    // would make a search hit unaddressable in the DOM.
    let c = compile(&schema(), &host());
    let hits = search(&c.root, "tags");
    assert_eq!(path_text(&hits.first().expect("a hit").path), "tags");
}
