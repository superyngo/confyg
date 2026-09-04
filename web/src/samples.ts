// The document the host boots with, so a fresh page is a form rather than an empty pane.
//
// It is not a new fixture. This is `confyg-session/examples/demo.{schema.json,toml}` —
// the pair `--example try` drives and `confyg-form/tests/snapshot.rs` pins — imported as
// text so there is exactly one copy. A sample that drifted from the fixtures the compiler
// is tested against would teach the wrong thing, and the deliberate edges are already
// there: an out-of-range `port` (an Invalid field), an unknown `colour` (preserved, not
// dropped), `tags` at its floor, `servers` at its ceiling, an absent optional `tls` Group,
// and a description whose wording its key never says.
//
// The Schema is dispatched as bytes, not as a `#:schema` hint: the session never fetches,
// and this host does not yet resolve the `SchemaFetchRequest` a hint would produce.
import schemaText from "../../crates/confyg-session/examples/demo.schema.json?raw";
import docText from "../../crates/confyg-session/examples/demo.toml?raw";

export const SAMPLE = {
  schemaName: "demo.schema.json",
  schemaText,
  docName: "demo.toml",
  docText,
  fmt: "Toml",
} as const;

// True while the open document is the built-in sample, which has no backing file. Opening
// or saving a real one drops the latch: `Save` must not offer to write back over a path
// the user never chose, and the chrome should stop calling it a sample once it isn't one.
let sampleMode = false;

export function inSampleMode(): boolean {
  return sampleMode;
}

export function setSampleMode(on: boolean): void {
  sampleMode = on;
  document.body.classList.toggle("sample", on);
}
