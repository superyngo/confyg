// Boot: load the WASM boundary, wire the four chrome actions, and render every snapshot
// the core hands back. The host owns file I/O and the theme; every form decision is the
// core's, arriving as a `SetterSnapshot`.
import { Boundary } from "./boundary.js";
import { formatFromName, initTheme, saveText, toggleTheme, type DocFormat } from "./host-io.js";
import { applyStaticI18n } from "./i18n.js";
import { render } from "./render.js";
import type { Ctx } from "./widgets/index.js";
import "./style.css";

const root = document.getElementById("form") as HTMLElement;
const openInput = document.getElementById("open") as HTMLInputElement;

let boundary: Boundary | null = null;
let fmt: DocFormat = "Toml";
let name = "config.toml";
let handle: FileSystemFileHandle | null = null;
let text = "";

type SnapshotOrError = Parameters<typeof render>[0] | { error: string };

// A malformed request comes back in the same envelope rather than trapping, so the host
// shows the message instead of losing the session.
function show(snapshot: SnapshotOrError): void {
  if ("error" in snapshot) {
    const message = document.createElement("p");
    message.className = "violation";
    message.textContent = snapshot.error;
    root.replaceChildren(message);
    return;
  }
  text = snapshot.text;
  render(snapshot, root, session);
}

// The controls' one way to write: a literal in, an intent out, and the snapshot that comes
// back is what the next render draws. `rawText` hands over bytes rather than JSON, so an
// unparseable literal is passed through as the string it is.
const session: Ctx = {
  set(path, literal) {
    if (!boundary) return;
    let value: unknown;
    try {
      value = JSON.parse(literal);
    } catch {
      value = literal;
    }
    // `SetterIntent` is internally tagged on `kind` (`confyg-ffi/tests/boundary.rs`).
    show(boundary.dispatch({ intent: { kind: "setValue", path, value } }));
  },
  unset(path) {
    if (!boundary) return;
    show(boundary.dispatch({ intent: { kind: "unset", path } }));
  },
};

initTheme();
applyStaticI18n();

document.getElementById("theme")?.addEventListener("click", toggleTheme);

openInput.addEventListener("change", async () => {
  const file = openInput.files?.[0];
  if (!file || !boundary) return;
  fmt = formatFromName(file.name);
  name = file.name;
  handle = null;
  show(
    boundary.dispatch({ command: { open: { text: await file.text(), fmt, path: file.name } } }),
  );
});

document.getElementById("save")?.addEventListener("click", async () => {
  if (!boundary) return;
  show(boundary.dispatch({ command: "save" }));
  handle = await saveText(text, fmt, name, handle);
});

// The session never fetches a Schema; the host does, and dispatches the bytes back.
document.getElementById("schema")?.addEventListener("click", () => {
  const picker = document.createElement("input");
  picker.type = "file";
  picker.accept = ".json";
  picker.addEventListener("change", async () => {
    const file = picker.files?.[0];
    if (!file || !boundary) return;
    show(
      boundary.dispatch({
        command: { loadSchema: { source: { Local: file.name }, text: await file.text() } },
      }),
    );
  });
  picker.click();
});

boundary = await Boundary.load();
