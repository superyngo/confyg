// Host-side I/O: the parts of a form session the browser owns rather than the core —
// reading a file in, writing it back out, and the theme attribute. Ported down from
// `confy/web/host-io.ts`, which carries the two-orchestrator `HostIo` surface, the VS Code
// channel, and the convert dialog; confyg v0.1 has one host and one Save, so only the flows
// it actually uses are here.
//
// The session never fetches: a `SchemaFetchRequest` in a snapshot is resolved by the host
// and dispatched back as `loadSchema`.

/** Capitalized because these are `confy_core::model::document::DocFormat`'s wire tags. */
export type DocFormat = "Toml" | "Json" | "Yaml";


const EXT: Record<DocFormat, string> = { Toml: ".toml", Json: ".json", Yaml: ".yaml" };

/** The Doc format a file name implies. Unrecognized extensions read as TOML, as upstream. */
export function formatFromName(name: string): DocFormat {
  const lower = name.toLowerCase();
  if (lower.endsWith(".json")) return "Json";
  if (lower.endsWith(".yaml") || lower.endsWith(".yml")) return "Yaml";
  return "Toml";
}

/**
 * Write `text` back to the open handle when there is one, else offer a destination, else
 * download. An in-place write to an already open handle never needs a picker — a picker
 * only chooses a *new* destination.
 */
export async function saveText(
  text: string,
  fmt: DocFormat,
  name: string,
  handle: FileSystemFileHandle | null,
): Promise<FileSystemFileHandle | null> {
  const suggestedName = name.endsWith(EXT[fmt]) ? name : name + EXT[fmt];
  let target = handle;
  // `showSaveFilePicker` is missing outside Chromium; that is the download path, not an error.
  if (!target && typeof window.showSaveFilePicker === "function") {
    target = await window.showSaveFilePicker({ suggestedName });
  }
  if (!target) {
    downloadText(text, suggestedName);
    return null;
  }
  const writable = await target.createWritable();
  await writable.write(text);
  await writable.close();
  return target;
}

function downloadText(text: string, name: string): void {
  const url = URL.createObjectURL(new Blob([text], { type: "text/plain" }));
  const link = document.createElement("a");
  link.href = url;
  link.download = name;
  link.click();
  URL.revokeObjectURL(url);
}

/**
 * `http://` → `https://` only when the page is itself https, where the browser would block
 * the plain-http fetch as mixed content.
 */
export function upgradeForMixedContent(url: string): string {
  if (typeof location !== "undefined" && location.protocol === "https:" && url.startsWith("http://")) {
    return "https://" + url.slice("http://".length);
  }
  return url;
}

// ---- theme (upstream's two `data-theme` attributes, unchanged) ----
type Theme = "dark" | "light";
const THEME_KEY = "confyg-theme";

export function initTheme(): void {
  let stored: string | null = null;
  try {
    stored = localStorage.getItem(THEME_KEY);
  } catch {
    // storage blocked — fall through to the media query
  }
  const prefersLight =
    typeof matchMedia !== "undefined" && matchMedia("(prefers-color-scheme: light)").matches;
  const theme: Theme = stored === "light" || stored === "dark" ? stored : prefersLight ? "light" : "dark";
  document.documentElement.dataset.theme = theme;
}

export function toggleTheme(): void {
  const next: Theme = document.documentElement.dataset.theme === "light" ? "dark" : "light";
  document.documentElement.dataset.theme = next;
  try {
    localStorage.setItem(THEME_KEY, next);
  } catch {
    // storage blocked — the attribute still holds for this session
  }
}
