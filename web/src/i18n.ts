// The Chassis lexicon: buttons, notices, widget names — confyg's own strings, versioned
// with confyg (presentation §6). Schema content (`title`, `description` of one Schema) is a
// separate carrier and is never merged into this catalog.
//
// Conventions are upstream's exactly (`confy/web/i18n.ts:60-89`): flat dot-delimited keys,
// `{0}` positional substitution, fallback active → `en` → the raw key, never throwing.
import en from "../../i18n/en.json";
import zhTw from "../../i18n/zh-TW.json";

export type Lang = "en" | "zh-TW";

const CATALOGS: Record<Lang, Record<string, string>> = {
  en: en as Record<string, string>,
  "zh-TW": zhTw as Record<string, string>,
};

export const LANG_DISPLAY_NAMES: Record<Lang, string> = {
  en: "English",
  "zh-TW": "繁體中文",
};

const STORAGE_KEY = "confyg-lang";

let currentLang: Lang | null = null;

export function getLang(): Lang {
  if (currentLang) return currentLang;
  let stored: string | null = null;
  try {
    stored = localStorage.getItem(STORAGE_KEY);
  } catch {
    // storage blocked (sandboxed webview) — detect from navigator instead
  }
  if (stored === "en" || stored === "zh-TW") {
    currentLang = stored;
  } else {
    const nl = typeof navigator !== "undefined" ? navigator.language : "en";
    currentLang = nl?.toLowerCase().startsWith("zh") ? "zh-TW" : "en";
  }
  return currentLang;
}

export function setLang(lang: Lang): void {
  currentLang = lang;
  try {
    localStorage.setItem(STORAGE_KEY, lang);
  } catch {
    // storage blocked — the in-memory choice still holds for this session
  }
}

/** `key` in the active language, falling back to `en`, then to the raw key. Never throws. */
export function t(key: string): string {
  const active = CATALOGS[getLang()][key];
  if (active !== undefined) return active;
  const fallback = CATALOGS.en[key];
  return fallback !== undefined ? fallback : key;
}

/** `t`, with `{0}`, `{1}`, … replaced positionally. Mirrors core's `tr_args`. */
export function tArgs(key: string, args: string[]): string {
  return t(key).replace(/\{(\d+)\}/g, (m, idx) => {
    const i = Number(idx);
    return i < args.length ? args[i] : m;
  });
}

/**
 * Apply `data-i18n` (textContent), `data-i18n-title`, and `data-i18n-placeholder` across
 * `root`. Snapshot-driven strings refresh through the dispatch round-trip; these static
 * labels do not come from a snapshot, so they are swept on boot and after a language change.
 */
export function applyStaticI18n(root: ParentNode = document): void {
  for (const node of root.querySelectorAll<HTMLElement>("[data-i18n]")) {
    if (node.dataset.i18n) node.textContent = t(node.dataset.i18n);
  }
  for (const node of root.querySelectorAll<HTMLElement>("[data-i18n-title]")) {
    if (node.dataset.i18nTitle) node.title = t(node.dataset.i18nTitle);
  }
  for (const node of root.querySelectorAll<HTMLInputElement>("[data-i18n-placeholder]")) {
    if (node.dataset.i18nPlaceholder) node.placeholder = t(node.dataset.i18nPlaceholder);
  }
}
