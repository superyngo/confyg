# Touch/desktop entry split — shared / reused / independent comparison
Status: Draft

The decision is [ADR 0006](../adr/0006-touch-desktop-dedicated-entry.md): touch gets a dedicated
entry (`touch.html` + `web/src/touch/`), not the `pointer: coarse` / `data-nav="push"` bolt-on
`web/src/shell.ts` ships today. This record is the inventory that decision needs: every current
`web/src/` module, whether it is coupled to "which shell is showing" today, and which of three
buckets it lands in once the split exists. It is written before implementation (`docs/spec/`
lifecycle); once shipped, its current-state half graduates into `docs/reference/`.

## 1. Why this is being evaluated now

`web/src/shell.ts`'s `wireNav()` decides `body[data-nav] = "push" | "columns"` from
`matchMedia("(pointer: coarse)")` and `(max-width: 720px)`; `web/src/style.css` carries both the
desktop three-column master-detail layout and the touch push-page/back-bar rules in one 894-line
cascade gated by that attribute (`style.css:773-888`, the "pushed-page shell" block). This shipped
2026-09-05 as part of the settings-template redesign (`docs/spec/20260904-redesign/PATCH.md`).

This is the pattern `wens-dev-principles ui 2` names as the failure mode to avoid — "Bolting touch
affordances onto desktop chrome behind `pointer: coarse` media checks produces a low-fidelity
hybrid that satisfies neither. This was tried once and fully reverted" — and it is not
hypothetical here: it is confyg's own upstream sibling's history. confy shipped exactly this
bolt-on, reverted it, and now routes to a dedicated `web/touch/` module from two HTML entries
(`docs/reference/WEBUI.md`'s "Touch UI" section in the confy repository). confy's
`docs/reference/TAURI.md` also records the concrete bug class a shared cascade invites: a
touch-only CSS fix once landed only in the desktop stylesheet because the two never shared a
cascade in the reverted design either — the same exposure confyg's single `style.css` carries as
its `data-nav="push"` block grows. This is not only an analogy: commit `15804f4`, the same
morning as this record, already had to restore a real-phone regression from that exact cascade —
a dropped `@import "./tokens.css"`, a missing `flex-direction`, and an inverted touch
grid-template-columns, all invisible on desktop and caught only by testing an emulated iPhone
viewport by hand.

Two facts calibrate how big this split needs to be, and both point at "much smaller than confy's
touch module," not "as large":

1. **confyg is a form, not a tree editor.** confy's `web/touch/` exists because confy has
   drag-and-drop, multi-select, a clipboard, and reorder gestures that behave differently under a
   finger. confyg has none of those — `repeat.ts`'s `RepeatCtx` offers only `addItem` /
   `removeItem` behind buttons. §4's inventory shows the renderer, widget registry, and CSS field
   language are already host-agnostic.
2. **The core already implements the one real degradation and no host can reach it.**
   `confyg-form::affordance::{Density, HostProfile}` (`Density::{Desktop, Phone, Touch}`) exists
   and is tested against a `Touch` profile in `confyg-session/tests/write_neutrality.rs` — but
   `confyg-ffi::Handle::new()` hardcodes `Session::new()` (`Density::Desktop`), so nothing built
   on this core has ever asked for anything else. See §6.

## 2. Terms used in this record

Not yet glossary entries (nothing here is shipped); on approval they migrate to
`docs/reference/glossary.md` per `wens-dev-principles docs`.

- **Entry**: one HTML document plus the script it boots (`index.html` + `main.ts`,
  `touch.html` + `web/src/touch/main.ts`). Exactly one entry is active per page load.
- **Shell** (as `shell.ts` already uses it): the chrome around the form — dirty state, violation
  chip, app preferences, navigation mode — never the form itself.
- **ShellHost**: the small interface `render.ts` and the boot sequence depend on instead of a
  concrete `Shell` class, so a `DesktopShell` and a `TouchShell` can each satisfy it.

## 3. Classification legend

| Category | Meaning |
|---|---|
| 共用 **Shared** | Identical module, imported verbatim by both entries. Zero fork, zero host branching inside it. |
| 沿用 **Reused/adapted** | The logic is one module, but a small seam (an interface, a container-agnostic "Surface") lets each host supply host-specific wiring without forking the logic. |
| 獨立實現 **Independent** | Genuinely separate file per host. No source is shared. |

## 4. Current-state inventory and target classification

| Module | Today's touch/desktop coupling | Target | Disposition |
|---|---|---|---|
| `boundary.ts` | None — pure WASM bridge | 共用 | Unchanged. Gains an optional `HostProfile` parameter to `Boundary.load()` (§6). |
| `types.ts` | None — wire-format mirror | 共用 | Unchanged. |
| `i18n.ts` | None | 共用 | Unchanged. |
| `host-io.ts` | None (file I/O + theme). Its `initTheme`/`toggleTheme` (`localStorage["confyg-theme"]`) are already dead code — superseded by `shell.ts`'s own `Prefs.theme`, unused since the redesign removed their last caller (CHANGELOG 2026-09-05) but never deleted; unrelated to this split. | 共用 | Unchanged. |
| `samples.ts` | None | 共用 | Unchanged. |
| `dom.ts` | None — pure element builders (`el`, `label`, `note`, `badge`, `button`, `chrome`, `groupHeading`) | 共用 | Unchanged. |
| `partition.ts` | None — Flow layer (`scroll` vs `sections`) decided from the Form IR alone | 共用 | Unchanged. |
| `search.ts` | None | 共用 | Unchanged. |
| `summary.ts` | None | 共用 | Unchanged. |
| `repeat.ts` | None — `RepeatCtx.renderChild` callback keeps it host-agnostic | 共用 | Unchanged. |
| `widgets/*.ts` (9 files incl. `common.ts`) | None — `mount(node, ctx)` pure functions; touch sizing already flows through `--hit`/`--row-h` tokens, not JS | 共用 | Unchanged. |
| `rowmenu.ts` | `attachLongPress` self-guards on `pointerType !== "mouse"`; `showRowMenu`/`toast()` query fixed element ids (`#toast`) | 共用 | Unchanged. Both host HTML shells keep a `#toast` element so `toast()` keeps working unmodified. |
| `render.ts` | Imports the concrete `shell` singleton and calls `appShell.afterRender()` (line 70), `appShell.showDetail()` (line 92), `appShell.sectionShown()` (line 116), `appShell.closeEntryDrawer()` (line 213) — the only coupling in an otherwise host-agnostic recursive Form IR walk | 沿用 | Phase 1: the four call sites take a `ShellHost` passed alongside `HostCtx` instead of importing the singleton. Every node-building function (`renderNode`/`renderGroup`/`renderField`/`renderRepeat`/`appendChildren`/`entryColumn`/`entryLabel`/`scrollLayout`/`sectionsLayout`/`detailFor`) stays exactly as it is. |
| `main.ts` | Boots `Boundary`, wires `HostCtx`, and does `document.getElementById` for ids that exist only in `index.html`; imports the concrete `shell` singleton directly | 沿用 | Phase 2: extract the Boundary-boot + `HostCtx`-wiring + save/open/schema/search glue into a shared `boot(refs, shellHost)` function. `main.ts` (desktop) and `touch/main.ts` each call it with their own element refs and `Shell` instance. |
| `shell.ts` | One class branches internally on `matchMedia("(pointer: coarse)")` / `(max-width: 720px)` to toggle `body[data-nav]`; owns both the columns/gear-popover behavior and the push/back-bar behavior | 獨立實現 | Phase 3: split into `DesktopShell` (today's columns default, media-query branch removed) and `TouchShell` (today's push default, promoted to unconditional). Both implement `ShellHost`. |
| `shell.ts`'s `appearance()`/`about()`/`pwa()`/`row()`/`segmented()`/`slider()` | Private methods building the app-preferences panel's DOM, entangled with the `Shell` class | 沿用 (recommended, optional) | Phase 4: extract to a container-agnostic `prefs-panel.ts`, mirroring confy's `panel.ts` / `convert-dialog.ts` Surface pattern, so `DesktopShell`'s popover and `TouchShell`'s pushed pane mount the same content builder. Not required for the split to function — both shells could keep short-term independent copies — but avoids forking preference UI logic in two. |
| `index.html` | One file carries the desktop title-bar chrome and the touch-only `#touch-bar`/`#touch-back` markup (`hidden` by default, shown by `shell.ts`) | 獨立實現 | Phase 2/3: `index.html` drops the touch-bar markup. A new `touch.html` carries touch's own chrome (back bar first, gear panel). Both keep the element ids `boot()` looks up: `#form`, `#toast`, `#results`, `#search`, `#save`, `#open`, `#schema`. |
| `style.css` | One 894-line cascade; ~115 lines (`style.css:773-888`, the "narrow width" and "pushed-page shell" blocks) are nav-conditional; the rest (field grid, widgets, summary, search, badges) is shared visual language | Split: 獨立實現 for the nav-conditional ~13%, 共用 for the rest | Phase 3: factor the shared ~87% into a base partial both `style.css` (desktop) and a new `web/src/touch/style.css` `@import`; delete the `data-nav` / `pointer: coarse` conditionals in favor of each file's own unconditional default. |
| `tokens.css` | Already host-agnostic; `--hit`/`--row-h` already vary by `@media (pointer: coarse)` | 共用 | Unchanged — this is the design's sanctioned "downward-only host/user preference on the Degradation ladder" (ADR 0004), not the bolt-on this record corrects. |
| `main.ts`'s service-worker registration | None | 共用 | Both entries register `/sw.js`. |
| *(new)* `entry-desktop.js` / `entry-touch.js` | — | 獨立實現 (paired) | New. Ported near-verbatim from confy's own two files (paths adjusted); the reverse-guard pattern (`?ui=` override, `matchMedia('(pointer:coarse)')` default) is the whole file in each case. |
| *(new)* `boot.ts` | — | 共用 (new home for `main.ts`'s host-agnostic half) | New. |
| *(new)* `shell-host.ts` (the `ShellHost` interface) | — | 共用 (contract only) | New. |

## 5. Target file tree

```
web/
  index.html                desktop entry — touch-bar markup removed
  touch.html                new — touch entry
  entry-desktop.js          new — router, first element in <head>
  entry-touch.js            new — reverse-guard router
  vite.config.ts            + a second Rollup HTML input
  src/
    boundary.ts              unchanged — shared
    types.ts                 unchanged — shared
    i18n.ts                  unchanged — shared
    host-io.ts                unchanged — shared
    samples.ts                unchanged — shared
    dom.ts                    unchanged — shared
    partition.ts               unchanged — shared
    search.ts                  unchanged — shared
    summary.ts                 unchanged — shared
    repeat.ts                  unchanged — shared
    widgets/*.ts                unchanged — shared
    rowmenu.ts                  unchanged — shared
    render.ts                   ShellHost param replaces the `shell` singleton import
    boot.ts                     new — extracted from main.ts, shared
    shell-host.ts                new — the ShellHost interface, shared
    prefs-panel.ts                new, optional (Phase 4) — shared content builder
    shell.ts                      → DesktopShell, implements ShellHost
    style.css                     desktop-only nav/chrome rules; @imports tokens.css + shared base
    main.ts                       thin — desktop refs + DesktopShell into boot()
    touch/
      main.ts                     new — thin — touch refs + TouchShell into boot()
      shell.ts                    new — TouchShell, implements ShellHost
      style.css                   new — touch-only nav/chrome rules; @imports tokens.css + shared base
```

Exact names are a proposal, not a requirement of the decision; the categorization in §4 is what
this record fixes. `web/src/touch/` (not a `web/touch/` sibling to `src/`) is used because
confyg's `web/` has no non-`src` TypeScript today, unlike confy's layout.

## 6. The core-side gap this split also closes

`confyg-form::affordance::{Density, HostProfile}` is implemented and tested — `Density::Touch`
and `Density::Phone` both appear in `confyg-session/tests/write_neutrality.rs`'s host-profile
matrix — but `confyg-ffi::Handle::new()` (`crates/confyg-ffi/src/lib.rs:30`) hardcodes
`Session::new()`, which is `Density::Desktop`. No web host, desktop or touch, has ever been able
to request anything else: this is core business logic that is built, tested, and unreachable.

Closing it is small and belongs with this split rather than after it, because the touch entry is
the first caller with a reason to ask for it:

- `confyg-ffi::Handle` gains a second constructor (or a constructor argument) accepting a
  `HostProfile` (or its `density` alone, if the other three fields stay hardcoded until a host
  actually needs to vary them).
- `boundary.ts`'s `Boundary.load()` takes an optional profile; desktop passes nothing (today's
  `Density::Desktop` default, unchanged), touch passes `Density::Touch`.
- This is additive FFI surface, not new core logic, and `write_neutrality.rs` already proves
  `Density` cannot change written bytes — so this cannot regress Write-neutrality (ADR 0004).

## 7. Migration phases

A sketch for the follow-up `docs/plan/` document this spec would produce once `Approved`; not
itself a task breakdown.

- **Phase 0.5 — factor `Shell.apply()`'s preference application out once.** `shell.ts`'s
  `Prefs.theme`/`rowHeight`/`textScale`/`lang` application (`apply()`) must be callable from both
  `DesktopShell` and `TouchShell` without copy-pasting it into each. (Unrelated pre-existing dead
  code, not this split's concern: `host-io.ts` still exports `initTheme`/`toggleTheme`, superseded
  by `shell.ts`'s `Prefs.theme` when the redesign removed their last caller — mention only, no
  action here.)
- **Phase 1 — decouple `render.ts`/`main.ts` from the concrete `shell` singleton** behind
  `ShellHost`. No visible behavior change; the existing `tests/e2e/first-run.spec.ts` must still
  pass unmodified.
- **Phase 2 — add the entry router.** `touch.html` + `entry-desktop.js` + `entry-touch.js` +
  the `vite.config.ts` multi-entry change; `touch.html` initially boots the *same* `main.ts` /
  `shell.ts` desktop shows, to prove the router alone before any touch-specific code exists.
- **Phase 3 — split the shell and the stylesheet.** `DesktopShell` / `TouchShell` behind
  `ShellHost`; `style.css` factored into a shared base plus two thin per-host files, deleting the
  `data-nav` / `pointer: coarse` conditionals.
- **Phase 4 (optional) — extract `prefs-panel.ts`** out of `Shell`, consumed by both shells.
- **Phase 5 — wire `Density::Touch`** through `confyg-ffi` and `boundary.ts` (§6) for the touch
  entry only.
- **Phase 6 — add Playwright coverage** for `touch.html` (a mobile-viewport project;
  `playwright.config.ts` has only `chromium` desktop today).
- **Phase 7 — promote to reference.** Once shipped, this record's §4 table graduates into
  `docs/reference/` (a new file, or a section of `presentation.md`) as the canonical current-state
  description, mirroring confy's `CHROME.md`/`WEBUI.md` pattern; this spec's `Status:` becomes
  `Shipped (YYYY-MM-DD)`.

## 8. Open decisions requiring confirmation before implementation starts

1. **"Touch-specific framework" reading.** This record and ADR 0006 read the request as a
   dedicated **vanilla TS/DOM module** — matching confy's own `web/touch/`, not a third-party UI
   framework. Confirm this before Phase 2.
2. **Module naming/layout** (`web/src/touch/` vs. a `web/touch/` sibling to `src/`) — §5's
   proposal is not load-bearing.
3. **Incremental landing vs. one branch.** Recommended: land Phases 0.5–6 as separate, individually
   testable commits/PRs against `main`, each independently revertable, rather than one large
   branch — matches this repository's existing one-task-per-commit convention.
4. **Timing relative to other open work** (e.g. the Unset-consolidation gap CHANGELOG's Known
   issues already track). Recommended: independent — no file overlap with this split.
5. **Phase 4 (`prefs-panel.ts` extraction) in scope now or deferred.** Not required for the split
   to work; a nice-to-have dedup.

## 9. Lifecycle

This is a Draft per `docs/spec/README.md`. On approval: flip to `Approved`, write
`docs/plan/2026-09-05-touch-desktop-split.md` breaking §7's phases into tasks, referencing
ADR 0006. Once shipped: flip to `Shipped (YYYY-MM-DD)`, and §4's table graduates into
`docs/reference/` per `wens-dev-principles docs`.
