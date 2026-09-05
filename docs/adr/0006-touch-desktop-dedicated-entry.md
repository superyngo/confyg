# 0006 — Touch gets a dedicated entry and shell, not a bolt-on responsive layer

Date: 2026-09-05

## Decision

confyg's touch experience ships as a **dedicated entry** (`touch.html` + a small `web/src/touch/`
module), routed to by a pointer-capability check that runs before first paint
(`entry-desktop.js` / `entry-touch.js`), rather than by branching the single `index.html` /
`main.ts` / `shell.ts` / `style.css` on `pointer: coarse` and viewport width, as the 2026-09-04
settings-template redesign shipped on 2026-09-05 currently does (`body[data-nav="push"]`,
`docs/spec/20260904-redesign/PATCH.md`).

Everything already host-agnostic stays exactly where it is and is imported by both entries
unchanged: the WASM boundary (`boundary.ts`), the Form IR types (`types.ts`), **Partition**
(`partition.ts`), the recursive renderer's node-building functions (`render.ts`), `repeat.ts`,
`summary.ts`, `search.ts`, `dom.ts`, `i18n.ts`, `host-io.ts`, `samples.ts`, the widget registry
(`widgets/*.ts`), and the row menu (`rowmenu.ts`, already pointer-type-guarded). Only the page
chrome — the navigation shell, the app-preferences panel's container, and the two HTML shells and
stylesheets that host them — forks per entry, behind one small `ShellHost` interface that
`render.ts` and an extracted boot sequence depend on instead of importing a concrete `shell`
singleton.

The full module-by-module disposition, target file tree, and migration phases are recorded in
[the companion spec](../spec/2026-09-05-touch-desktop-split.md); this ADR records the decision
and why, not the inventory.

## Why

`wens-dev-principles ui 2` names the failure mode by name: "Bolting touch affordances onto
desktop chrome behind `pointer: coarse` media checks produces a low-fidelity hybrid that
satisfies neither. This was tried once and fully reverted." That is not a hypothetical for this
family of projects — it is confyg's own upstream sibling's history.
[confy](https://github.com/superyngo/confy)'s `docs/reference/WEBUI.md` states it directly: "The
touch experience is **not** the desktop UI with gestures bolted on — that was tried and rejected
as low-fidelity. Instead `web/touch/` is a **separate, prototype-faithful UI**." confy replaced
the bolt-on with two HTML entry points (`index.html` / `touch.html`), a pointer-based two-script
router (`entry-desktop.js` / `entry-touch.js`), and a dedicated `web/touch/` module — while
sharing every core and business-logic module verbatim (`confy.ts`, `types.ts`, `fs.ts`,
`key-intent.ts`, `panel.ts`, `convert-dialog.ts`, `typefilter.ts`, `i18n.ts`, and more) via plain
imports, with **zero third-party UI framework** on either side.

confyg shipped the exact pattern confy reverted: `web/src/shell.ts`'s `wireNav()` toggles
`body[data-nav]` from `matchMedia("(pointer: coarse)")` / `(max-width: 720px)`, and
`web/src/style.css` carries both the desktop three-column layout and the touch push-page/back-bar
rules in one 894-line cascade gated by that attribute (`style.css:773-888`). confy's own
`docs/reference/TAURI.md` records the concrete failure mode a shared cascade invites: a
touch-only CSS fix once landed only in the desktop stylesheet because "the two are not the same
cascade and nothing here shares rules between them by default" — the same exposure confyg's
single `style.css` carries today as its `data-nav="push"` block grows — and this is not only an
analogy: commit `15804f4` (the same morning as this ADR) already had to restore a real-phone
regression from that exact cascade — a dropped `@import "./tokens.css"`, a missing
`flex-direction`, and an inverted touch grid-template-columns, all in the one shared
`style.css` the redesign introduced, all invisible on desktop and only caught by testing an
emulated iPhone viewport by hand.

Two facts keep this proportionate rather than a wholesale copy of confy's touch module:

1. **confyg is a form, not a tree editor.** confy's touch module exists because confy has
   drag-and-drop, multi-select, a clipboard, and reorder gestures that all behave differently
   under a finger. confyg has none of those — `repeat.ts`'s `RepeatCtx` offers only `addItem` /
   `removeItem` behind buttons, with no move/reorder at all. The renderer, the widget registry,
   and the CSS field/section language are therefore already host-agnostic today, which the
   companion spec's inventory confirms module by module.
2. **The core already models the one real touch-vs-desktop degradation and it is unreachable.**
   `confyg-form::affordance::{Density, HostProfile}` (`Density::{Desktop, Phone, Touch}`) is
   implemented and exercised by `confyg-session/tests/write_neutrality.rs` against a `Touch`
   profile today — but `confyg-ffi::Handle::new()` hardcodes `Session::new()`
   (`Density::Desktop`), so no web host, desktop or touch, can ever request anything else. The
   dedicated touch entry is what finally has a reason to call a `with_host`-shaped constructor
   with `Density::Touch`; the desktop entry keeps `Density::Desktop` and is provably unaffected
   (`write_neutrality.rs` already proves `Density` cannot change written bytes).

## Alternatives rejected

- **Keep bolting touch onto desktop chrome** via `pointer: coarse` / width media queries and a
  `data-nav` attribute (status quo, shipped 2026-09-05). Rejected: this is the exact pattern
  confy tried and reverted, for the exact reason above, and the shared-cascade failure mode is
  not speculative — confy hit it in production.
- **A full confy-style separate touch module**, including its own renderer, gesture layer, and a
  prototype-ported visual design duplicating the whole CSS visual language. Rejected as
  over-scoped: confyg has no pointer-gesture surface to port, so duplicating the renderer, widget
  registry, and shared visual language would recreate, inside one repository, the exact
  maintenance cost ADR 0001 already rejected for a different pair of codebases.
- **A third-party touch/mobile UI framework** (e.g. Framework7, Ionic). Rejected: confy's own
  touch UI — the richest touch surface in this project family, gestures included — ships zero
  framework dependencies; confyg's entire stack is deliberately framework-free
  (`web/package.json`); and every form decision must still originate from the WASM-driven
  `SetterSnapshot` ("the host makes no form decisions," `docs/reference/presentation.md`), which a
  component framework's own state/diffing model would sit beside without adding any capability
  the existing DOM-builder functions lack.

## Consequences

- `web/vite.config.ts` gains a second Rollup HTML input (`touch.html`) — small and additive.
- `render.ts` and `main.ts` lose their direct imports of the concrete `shell` singleton; the four
  call sites (`appShell.afterRender/showDetail/sectionShown/closeEntryDrawer`) move to a small
  `ShellHost` interface that a `DesktopShell` and a `TouchShell` each implement — mechanical, and
  no behavior change on desktop.
- `confyg-ffi::Handle` needs a second constructor (or a constructor argument) accepting a
  `HostProfile`/`Density`, so `Boundary.load()` can request `Density::Touch` for the touch entry.
  This is new FFI surface, not new core logic — `affordance.rs` and `write_neutrality.rs` already
  cover `Density::Touch`.
- Two HTML files and two small per-host TS/CSS files now exist and must be kept from drifting the
  way confy's `docs/reference/CHROME.md` exists to prevent; a comparable table belongs in
  `docs/reference/` once this ships (see the companion spec's lifecycle note).
- `playwright.config.ts` has one `chromium` desktop project today; a touch/mobile-viewport project
  against `touch.html` is new coverage this decision creates a need for, not something it ships
  by itself.
- `shell.ts`'s theme/density/text-scale preference application (`Shell.apply()`) needs to be
  callable from both `DesktopShell` and `TouchShell` without being copy-pasted into each — it
  should be factored out once, not forked twice. Unrelated pre-existing dead code, noted but out
  of scope here: `host-io.ts` still exports `initTheme`/`toggleTheme`
  (`localStorage["confyg-theme"]`), superseded by `shell.ts`'s own `Prefs.theme`
  (`localStorage["confyg.prefs"]`) when the redesign removed their last caller
  (CHANGELOG 2026-09-05, "Removed the now-dead `#theme` button wiring... since `shell.ts` owns
  theme") but never deleted.
