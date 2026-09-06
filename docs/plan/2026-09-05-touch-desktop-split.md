# Touch/desktop entry split — implementation plan
Status: In progress — 2/7 tasks

> **For agentic workers:** REQUIRED SUB-SKILL: use `subagent-driven-development` (recommended) or
> `executing-plans` to implement this plan task by task. Steps use checkbox (`- [ ]`) syntax for
> tracking. Every task ends with a commit; per `CLAUDE.md`, that commit carries the code, the
> `CHANGELOG.md` entry, and any documentation update together — one task, one self-contained
> commit, matching this repository's existing one-task-per-commit convention (spec §8.3).

**Goal:** give touch its own dedicated entry (`touch.html` + `web/src/touch/`), routed to before
first paint, replacing the `pointer: coarse` / `data-nav="push"` bolt-on `web/src/shell.ts` ships
today — per [ADR 0006](../adr/0006-touch-desktop-dedicated-entry.md).

**Spec:** [`../spec/2026-09-05-touch-desktop-split.md`](../spec/2026-09-05-touch-desktop-split.md)
(module-by-module disposition §4, target file tree §5, core-side gap §6, confirmed decisions §8).
Cited below as *spec §n*. Vocabulary: [`../reference/glossary.md`](../reference/glossary.md) —
`Host entry`/`ShellHost` are not yet promoted there (spec §9); this plan cites the spec's own §2
for those terms until the split ships.

Tasks below are a direct breakdown of spec §7's phases; task numbering matches phase numbering.
Phase 4 (`prefs-panel.ts` extraction) is confirmed deferred (spec §8.5) and is **not** a task in
this plan — revisit as a follow-up plan once Tasks 0.5–3 and 5–6 have shipped and the duplication
is visible in both shells.

---

## Task 0.5: Factor `Shell.apply()`'s preference application out once

**Files:**
- Edit: `web/src/shell.ts`

**Interfaces:**
- Produces: a standalone function (or exported method) applying `Prefs.theme`/`rowHeight`/
  `textScale`/`lang` to the DOM, callable without a full `Shell` instance, for `DesktopShell` and
  `TouchShell` (Task 3) to both call.

- [x] **Step 1:** Extract today's `Shell.apply()` body into a function taking the DOM refs/prefs
      it needs as parameters, not `this`. `Shell.apply()` becomes a thin wrapper calling it.
- [x] **Step 2:** Run `tests/e2e/first-run.spec.ts` — must pass unmodified; no visible behavior
      change on desktop.
- [x] Commit: code + CHANGELOG entry. Do not touch `host-io.ts`'s dead `initTheme`/`toggleTheme` —
      out of scope (spec §7 Phase 0.5 note).

## Task 1: Decouple `render.ts`/`main.ts` from the concrete `shell` singleton

**Files:**
- Create: `web/src/shell-host.ts` (the `ShellHost` interface — exactly 4 methods, spec §2/§4)
- Edit: `web/src/render.ts`, `web/src/main.ts`

**Interfaces:**
- Produces: `ShellHost { afterRender(); showDetail(...); sectionShown(...); closeEntryDrawer(); }`.
- Consumes: nothing new — `web/src/shell.ts`'s existing `Shell` class must satisfy `ShellHost`
  structurally (or via an explicit `implements`) without changing its own behavior.

- [x] **Step 1:** Write `shell-host.ts` with the 4-method interface, typed from today's call
      sites (`render.ts:70,92,116,213` per spec §4's `render.ts` row).
- [x] **Step 2:** Threaded `ShellHost` alongside `HostCtx` via `render()`'s existing
      module-level-singleton pattern (mirrors how `HostCtx` itself already reaches
      `renderNode`/`renderField`/etc. through the module-level `active`, not per-call
      parameters) — a `host?: ShellHost` param on `render()` sets a module-level `activeHost`,
      which `reveal`/`scrollLayout`/`sectionsLayout` etc. read instead of the removed
      `appShell` import. None of their own logic changed, only how they reach `ShellHost`.
- [x] **Step 3:** `main.ts` passes its concrete `shell` singleton as the `ShellHost` argument
      instead of `render.ts` importing it directly.
- [x] **Step 4:** Run `tests/e2e/first-run.spec.ts` — must pass unmodified (spec §7 Phase 1
      explicit acceptance).
- [x] Commit: code + CHANGELOG entry.

## Task 2: Add the entry router

**Files:**
- Create: `web/touch.html`, `web/entry-desktop.js`, `web/entry-touch.js`
- Edit: `web/vite.config.ts`, `web/public/sw.js`

**Interfaces:**
- Produces: `touch.html` (initially boots the *same* `main.ts`/`shell.ts` desktop shows — spec §7
  Phase 2 — to prove the router alone before touch-specific code exists).
- Consumes: none new.

- [ ] **Step 1:** Port `entry-desktop.js`/`entry-touch.js` near-verbatim from confy's own two
      files (paths adjusted) — the reverse-guard pattern (`?ui=` override, undocumented dev-only
      per spec §8.7; `matchMedia('(pointer:coarse)')` default) is the whole file in each case.
- [ ] **Step 2:** Add `touch.html` pointing at `main.ts` unchanged (no `touch/` module yet).
- [ ] **Step 3:** Add the second Rollup HTML input to `vite.config.ts`.
- [ ] **Step 4:** Add `"/touch.html"` to `public/sw.js`'s `SHELL` precache array (spec §4's
      service-worker row). Leave `manifest.webmanifest`'s `start_url`/`scope` as `/` (spec §8.7 —
      decided, not revisited here).
- [ ] **Step 5:** Manually verify `/touch.html` loads and behaves identically to `/` (same
      `main.ts`/`shell.ts`); run `tests/e2e/first-run.spec.ts` against `/` — must still pass
      unmodified.
- [ ] Commit: code + CHANGELOG entry.

## Task 3: Split the shell and the stylesheet

**Files:**
- Create: `web/src/touch/main.ts`, `web/src/touch/shell.ts`, `web/src/touch/style.css`
- Edit: `web/src/shell.ts` (→ `DesktopShell`), `web/src/style.css`, `web/touch.html`
  (points at `web/src/touch/main.ts`), `web/src/boot.ts` (if not yet extracted — see note below)

**Interfaces:**
- Produces: `DesktopShell` and `TouchShell`, both implementing `ShellHost` (Task 1).
- Consumes: `ShellHost` (Task 1), the extracted `apply()` (Task 0.5).

- [ ] **Step 1:** Extract `main.ts`'s host-agnostic half (Boundary-boot + `HostCtx`-wiring +
      save/open/schema/search glue) into `boot(refs, shellHost)` in a new `web/src/boot.ts`, per
      spec §7 Phase 2's original scope note folded in here since `TouchShell` needs it now.
      `main.ts` (desktop) becomes thin, calling `boot()` with desktop refs + `DesktopShell`.
- [ ] **Step 2:** Split `shell.ts`'s `wireNav()` media-query branch: `DesktopShell` keeps today's
      columns default with the branch removed; new `web/src/touch/shell.ts`'s `TouchShell`
      promotes today's push/back-bar behavior to unconditional. Both implement `ShellHost` and
      call the Task 0.5 `apply()` function.
- [ ] **Step 3:** `web/src/touch/main.ts` — thin, touch refs + `TouchShell` into `boot()`.
- [ ] **Step 4:** `touch.html` drops the touch-bar markup requirement on `index.html`; `index.html`
      drops its `#touch-bar`/`#touch-back` markup entirely (spec §4's `index.html` row). Both
      keep `#form`, `#toast`, `#results`, `#search`, `#save`, `#open`, `#schema`.
- [ ] **Step 5:** Factor `style.css`'s shared ~87% (field grid, widgets, summary, search, badges)
      into a base partial; `style.css` (desktop) and new `web/src/touch/style.css` each
      `@import` it plus `tokens.css`, then their own unconditional nav/chrome rules — deleting
      the `data-nav`/`pointer: coarse` conditionals (`style.css:773-888` today).
- [ ] **Step 6:** Run `tests/e2e/first-run.spec.ts` against `/` — must pass unmodified (no
      desktop behavior change). Manually verify `/touch.html` now shows push/back-bar chrome by
      default, no media query involved.
- [ ] Commit: code + CHANGELOG entry.

## Task 5: Wire `Density::Touch` through `confyg-ffi` and `boundary.ts`

**Files:**
- Edit: `crates/confyg-ffi/src/lib.rs`, `web/src/boundary.ts`, `web/src/touch/main.ts`

**Interfaces:**
- Produces: `confyg-ffi::Handle` gains a second constructor (or constructor argument) accepting a
  `HostProfile`/`density` (spec §6). Only `density` varies — `can_mask`/`can_slide`/
  `can_filter_options` are copied verbatim from today's Desktop default (`true`/`true`/`false`),
  per spec §8.6. Real touch-appropriate values for those three are explicitly out of scope here.
- Consumes: `confyg-form::affordance::{Density, HostProfile}` (already implemented/tested).

- [ ] **Step 1:** Add the `HostProfile`-accepting constructor to `confyg-ffi::Handle`, keeping
      `Handle::new()`'s existing `Density::Desktop` default constructor untouched so desktop's
      call site needs zero changes.
- [ ] **Step 2:** `boundary.ts`'s `Boundary.load()` takes an optional profile parameter; desktop
      passes nothing (unchanged), `web/src/touch/main.ts` passes `Density::Touch` with the other
      three flags matching Desktop's values.
- [ ] **Step 3:** Run `cargo test -p confyg-session write_neutrality` — must still pass (proves
      `Density` cannot change written bytes, so this cannot regress Write-neutrality per ADR
      0004, spec §6).
- [ ] **Step 4:** Manually verify touch entry loads with `Density::Touch` wired (no visible
      behavior change expected yet, since the other three flags are unchanged from Desktop).
- [ ] Commit: code + CHANGELOG entry.

## Task 6: Add Playwright coverage for the touch entry

**Files:**
- Edit: `web/playwright.config.ts`
- Create: none — full parity, not a separate touch-only spec file (spec §7 Phase 6 / §8 grill
  decision)

**Interfaces:**
- Produces: a second Playwright project (mobile viewport) alongside today's `chromium` desktop
  project.
- Consumes: `tests/e2e/first-run.spec.ts` unmodified — runs against both projects because spec §4
  guarantees identical element ids (`#form`, `#toast`, etc.) on both HTML shells.

- [ ] **Step 1:** Add a mobile-viewport Playwright project (e.g. `Pixel 5` or similar device
      preset) targeting `/touch.html` to `playwright.config.ts`.
- [ ] **Step 2:** Run `tests/e2e/first-run.spec.ts` against both projects — must pass unmodified
      on both, proving parity and that the split didn't silently change desktop behavior.
- [ ] Commit: code + CHANGELOG entry.

## Task 7: Promote to reference

**Files:**
- Create: a new `docs/reference/` file, or a section of `docs/reference/presentation.md`
  (decide at task time which fits better, mirroring confy's `CHROME.md`/`WEBUI.md` pattern)
- Edit: `docs/spec/2026-09-05-touch-desktop-split.md` (`Status:` → `Shipped (YYYY-MM-DD)`),
  `docs/reference/glossary.md` (promote `Host entry`, `ShellHost`, `Shell` — spec §2)
- Edit: `docs/plan/README.md` (move this plan from "In progress" to "Landed")

- [ ] **Step 1:** Write the canonical current-state description from spec §4's table (now
      accurate to what actually shipped, which may have drifted from the plan in small ways —
      reconcile against the real files, don't just copy the spec).
- [ ] **Step 2:** Promote `Host entry`/`ShellHost`/`Shell` glossary entries per spec §9's
      lifecycle rule (only now, not earlier).
- [ ] **Step 3:** Flip spec `Status:` to `Shipped (YYYY-MM-DD)`; update this plan's own `Status:`
      to `Shipped`; move its `docs/plan/README.md` row to "Landed".
- [ ] Commit: docs only + CHANGELOG entry.
