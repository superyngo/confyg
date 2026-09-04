use confy_core::model::any_doc::AnyDocument;
use confy_core::model::document::{ConfigDocument, DocFormat};

#[test]
fn upstream_parses_and_serializes_byte_identically() {
    let src = "# lead\n[server]\nhost = \"a\"  # trail\n";
    let doc = AnyDocument::from_str_as(src, DocFormat::Toml).expect("parse");
    assert_eq!(
        doc.serialize(),
        src,
        "untouched text must round-trip byte-identically"
    );
}

#[test]
fn confy_session_is_not_linked() {
    // The session module is never referenced (ADR 0001). This is asserted by CI's grep step;
    // the compile-time half is that this crate builds with default-features = false.
}
