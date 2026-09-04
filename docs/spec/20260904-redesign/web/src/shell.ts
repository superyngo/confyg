// The chrome around the form: what document is open, whether it is written, how many
// things are wrong, the application's own preferences, and which of the two navigation
// shells is in use.
//
// It owns no form state. Every decision about a field belongs to the core; everything here
// is about the window — which is why appearance, language, diagnostics and installation
// live behind one gear and never in the section list. The section list is for the file.
import { t } from "./i18n.js";
import { toast } from "./rowmenu.js";

type Pane = "appearance" | "about" | "pwa";

const PREFS = "confyg.prefs";

interface Prefs {
  theme: "system" | "light" | "dark";
  /** Control height in px, 28–52. Presentation only; never written to the document. */
  rowHeight: number;
  /** 0.9–1.3. */
  textScale: number;
  lang: string;
}

const fallback: Prefs = { theme: "system", rowHeight: 30, textScale: 1, lang: "en" };

function read(): Prefs {
  try {
    const raw = localStorage.getItem(PREFS);
    return raw ? { ...fallback, ...(JSON.parse(raw) as Partial<Prefs>) } : { ...fallback };
  } catch {
    return { ...fallback };
  }
}

class Shell {
  private prefs = read();
  private dirty = false;
  private install: Event | null = null;
  private page: "list" | "detail" = "list";

  init(): void {
    this.apply();
    this.wireChrome();
    this.wireNav();
    window.addEventListener("beforeinstallprompt", (e) => {
      e.preventDefault();
      this.install = e;
    });
    // Unsaved work must not vanish on a reload of an installed window.
    window.addEventListener("beforeunload", (e) => {
      if (this.dirty) e.preventDefault();
    });
  }

  /** Write the presentation prefs as tokens. Nothing here touches the document. */
  private apply(): void {
    const root = document.documentElement;
    const dark =
      this.prefs.theme === "system"
        ? window.matchMedia("(prefers-color-scheme: dark)").matches
        : this.prefs.theme === "dark";
    root.dataset.theme = dark ? "dark" : "light";
    root.style.setProperty("--row-h", this.prefs.rowHeight + "px");
    root.style.setProperty("--font-scale", String(this.prefs.textScale));
    root.lang = this.prefs.lang;
    try {
      localStorage.setItem(PREFS, JSON.stringify(this.prefs));
    } catch {
      /* A private window still gets a working session, just no memory of it. */
    }
  }

  private set<K extends keyof Prefs>(key: K, value: Prefs[K]): void {
    this.prefs[key] = value;
    this.apply();
    if (key === "lang") window.location.reload();
  }

  // ---------- chrome ----------

  private wireChrome(): void {
    const gear = document.getElementById("app-menu");
    const panel = document.getElementById("app-panel");
    if (gear && panel) {
      gear.addEventListener("click", () => {
        const open = panel.hidden;
        panel.hidden = !open;
        gear.setAttribute("aria-expanded", String(open));
      });
      for (const item of panel.querySelectorAll<HTMLButtonElement>("button[data-pane]")) {
        item.addEventListener("click", () => {
          panel.hidden = true;
          gear.setAttribute("aria-expanded", "false");
          this.showPane(item.dataset.pane as Pane);
        });
      }
      document.addEventListener("click", (e) => {
        if (panel.hidden) return;
        if (e.target instanceof Node && (panel.contains(e.target) || gear.contains(e.target))) return;
        panel.hidden = true;
        gear.setAttribute("aria-expanded", "false");
      });
    }
    // The violation chip is the way back to a summary the user dismissed.
    document.getElementById("issues")?.addEventListener("click", () => {
      const summary = document.querySelector<HTMLElement>("#form .summary");
      if (summary) summary.hidden = false;
      this.afterRender();
    });
  }

  /** The document is what the title bar is about: name, format, and whether it is written. */
  markClean(name: string, fmt: string): void {
    this.dirty = false;
    const n = document.getElementById("file-name");
    const f = document.getElementById("file-fmt");
    if (n) n.textContent = name;
    if (f) f.textContent = " \u00b7 " + fmt.toUpperCase();
    const chip = document.getElementById("dirty");
    if (chip) chip.hidden = true;
    const save = document.getElementById("save");
    if (save) save.textContent = t("form.state.saved");
  }

  markDirty(): void {
    if (this.dirty) return;
    this.dirty = true;
    const chip = document.getElementById("dirty");
    if (chip) chip.hidden = false;
    const save = document.getElementById("save");
    if (save) save.textContent = t("form.action.save");
  }

  /**
   * Called at the end of every render. The count comes from the rendered document, not the
   * snapshot: one source of truth, and it is the one the user can see.
   */
  afterRender(): void {
    const count = document.querySelectorAll("#form .violation").length;
    const chip = document.getElementById("issues");
    const summary = document.querySelector<HTMLElement>("#form .summary");
    const dismissed = !summary || summary.hidden;
    if (chip) {
      chip.textContent = String(count) + " " + t("form.summary.toFix");
      chip.hidden = count === 0 || !dismissed;
    }
    this.measure();
  }

  // ---------- adaptation ----------

  private wireNav(): void {
    const coarse = window.matchMedia("(pointer: coarse)");
    const narrow = window.matchMedia("(max-width: 720px)");
    const decide = (): void => {
      document.body.dataset.nav = coarse.matches || narrow.matches ? "push" : "columns";
      document.body.dataset.page = this.page;
      const bar = document.getElementById("touch-bar");
      if (bar) bar.hidden = document.body.dataset.nav !== "push";
      this.measure();
    };
    coarse.addEventListener("change", decide);
    narrow.addEventListener("change", decide);
    window.addEventListener("resize", () => this.measure());
    document.getElementById("touch-back")?.addEventListener("click", () => this.showList());
    decide();
  }

  /**
   * The entry column is a column while the detail pane can still hold a form, and a drawer
   * once it cannot. 520px is where a label track plus a value stops fitting.
   */
  private measure(): void {
    const shell = document.querySelector<HTMLElement>(".form-shell");
    if (!shell) return;
    const sidebar = shell.querySelector<HTMLElement>(".section-list")?.offsetWidth ?? 0;
    const column = shell.querySelector<HTMLElement>(".entry-column");
    if (!column) return;
    const room = shell.offsetWidth - sidebar - column.offsetWidth;
    const drawer = document.body.dataset.nav === "columns" && room < 520;
    document.body.dataset.entry = drawer ? "drawer" : "column";
    if (drawer) {
      column.hidden = true;
      this.ensureDrawerButton(shell, column);
    } else {
      column.hidden = false;
      shell.querySelector(".entry-drawer-open")?.remove();
      shell.querySelector(".scrim")?.remove();
    }
  }

  private ensureDrawerButton(shell: HTMLElement, column: HTMLElement): void {
    const detail = shell.querySelector<HTMLElement>(".section-detail");
    if (!detail || detail.querySelector(".entry-drawer-open")) return;
    const open = document.createElement("button");
    open.type = "button";
    open.className = "entry-drawer-open";
    open.textContent = (column.querySelector(".entry-column-title")?.textContent ?? "") || t("form.repeat.entries");
    open.addEventListener("click", () => {
      column.hidden = false;
      const scrim = document.createElement("button");
      scrim.type = "button";
      scrim.className = "scrim";
      scrim.addEventListener("click", () => this.closeEntryDrawer());
      shell.append(scrim);
    });
    detail.prepend(open);
  }

  closeEntryDrawer(): void {
    if (document.body.dataset.entry !== "drawer") return;
    const column = document.querySelector<HTMLElement>(".entry-column");
    if (column) column.hidden = true;
    document.querySelector(".scrim")?.remove();
  }

  /** The pushed shell: the section list is a page, a section is pushed on top of it. */
  showDetail(): void {
    this.page = "detail";
    document.body.dataset.page = "detail";
  }

  showList(): void {
    this.page = "list";
    document.body.dataset.page = "list";
    const pane = document.getElementById("app-pane");
    const form = document.getElementById("form");
    if (pane && form) {
      pane.hidden = true;
      form.hidden = false;
    }
  }

  sectionShown(title: string): void {
    const label = document.getElementById("touch-title");
    if (label) label.textContent = title;
    if (document.body.dataset.nav === "push") this.showDetail();
  }

  // ---------- the application's own settings ----------

  private showPane(pane: Pane): void {
    const host = document.getElementById("app-pane");
    const form = document.getElementById("form");
    if (!host || !form) return;
    host.replaceChildren(pane === "appearance" ? this.appearance() : pane === "about" ? this.about() : this.pwa());
    host.hidden = false;
    form.hidden = true;
    const label = document.getElementById("touch-title");
    if (label) label.textContent = t("form.app." + pane);
    this.showDetail();
  }

  private row(labelKey: string, control: HTMLElement): HTMLElement {
    const row = document.createElement("div");
    row.className = "pref-row";
    const name = document.createElement("span");
    name.textContent = t(labelKey);
    row.append(name, control);
    return row;
  }

  private segmented(options: Array<[string, string]>, chosen: string, pick: (v: string) => void): HTMLElement {
    const box = document.createElement("div");
    box.className = "segmented";
    for (const [value, key] of options) {
      const b = document.createElement("button");
      b.type = "button";
      b.textContent = t(key);
      b.setAttribute("aria-pressed", String(value === chosen));
      b.addEventListener("click", () => {
        pick(value);
        for (const other of box.querySelectorAll("button")) {
          other.setAttribute("aria-pressed", String(other === b));
        }
      });
      box.append(b);
    }
    return box;
  }

  private slider(min: number, max: number, value: number, step: number, onInput: (v: number) => void): HTMLElement {
    const box = document.createElement("div");
    box.className = "control";
    const input = document.createElement("input");
    input.type = "range";
    input.min = String(min);
    input.max = String(max);
    input.step = String(step);
    input.value = String(value);
    const read = document.createElement("span");
    read.className = "entry-count";
    read.textContent = String(value);
    input.addEventListener("input", () => {
      const v = Number(input.value);
      read.textContent = String(v);
      onInput(v);
    });
    box.append(input, read);
    return box;
  }

  private appearance(): HTMLElement {
    const pane = document.createElement("section");
    const h = document.createElement("h2");
    h.className = "node-heading";
    h.textContent = t("form.app.appearance");
    pane.append(h);
    pane.append(
      this.row(
        "form.pref.theme",
        this.segmented(
          [["system", "form.pref.themeSystem"], ["light", "form.pref.themeLight"], ["dark", "form.pref.themeDark"]],
          this.prefs.theme,
          (v) => this.set("theme", v as Prefs["theme"]),
        ),
      ),
      this.row("form.pref.density", this.slider(28, 52, this.prefs.rowHeight, 1, (v) => this.set("rowHeight", v))),
      this.row("form.pref.textSize", this.slider(90, 130, Math.round(this.prefs.textScale * 100), 1, (v) => this.set("textScale", v / 100))),
      this.row(
        "form.pref.language",
        this.segmented([["en", "form.pref.langEn"], ["zh-TW", "form.pref.langZh"]], this.prefs.lang, (v) => this.set("lang", v)),
      ),
    );
    const note = document.createElement("p");
    note.className = "description";
    note.textContent = t("form.pref.note");
    pane.append(note);
    return pane;
  }

  private about(): HTMLElement {
    const pane = document.createElement("section");
    const h = document.createElement("h2");
    h.className = "node-heading";
    h.textContent = t("form.app.about");
    pane.append(h);
    // Every line is a fact the host already knows; nothing here is computed for display.
    const facts: Array<[string, string]> = [
      ["form.diag.schema", document.getElementById("file-name")?.dataset.schema ?? t("form.diag.none")],
      ["form.diag.validator", document.body.dataset.validator ?? t("form.diag.available")],
      ["form.diag.file", (document.getElementById("file-name")?.textContent ?? "") + (document.getElementById("file-fmt")?.textContent ?? "")],
      ["form.diag.degraded", String(document.querySelectorAll("#form [data-intended]").length)],
      ["form.diag.offline", navigator.onLine ? t("form.diag.online") : t("form.diag.offlineYes")],
    ];
    for (const [key, value] of facts) {
      const line = document.createElement("div");
      line.className = "pref-row";
      const name = document.createElement("span");
      name.textContent = t(key);
      const v = document.createElement("span");
      v.className = "raw-preview";
      v.textContent = value;
      line.append(name, v);
      pane.append(line);
    }
    const out = document.createElement("button");
    out.type = "button";
    out.className = "entry-drawer-open";
    out.textContent = t("form.diag.export");
    out.addEventListener("click", () => {
      const blob = new Blob([JSON.stringify(Object.fromEntries(facts.map(([k, v]) => [k, v])), null, 2)], {
        type: "application/json",
      });
      const a = document.createElement("a");
      a.href = URL.createObjectURL(blob);
      a.download = "confyg-diagnostics.json";
      a.click();
      URL.revokeObjectURL(a.href);
    });
    pane.append(out);
    return pane;
  }

  private pwa(): HTMLElement {
    const pane = document.createElement("section");
    const h = document.createElement("h2");
    h.className = "node-heading";
    h.textContent = t("form.app.pwa");
    const why = document.createElement("p");
    why.className = "description";
    why.textContent = t("form.pwa.why");
    const action = document.createElement("button");
    action.type = "button";
    action.className = "entry-drawer-open";
    action.textContent = t("form.pwa.install");
    action.addEventListener("click", () => {
      const prompt = this.install as (Event & { prompt?: () => Promise<void> }) | null;
      if (prompt?.prompt) void prompt.prompt();
      else toast(t("form.pwa.already"));
    });
    pane.append(h, why, action);
    return pane;
  }
}

export const shell = new Shell();
