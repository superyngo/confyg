# Backlog
Status: In progress

The one living record of open work. Rows move to Done with the commit that closed them and are
never deleted. Evidence is file + symbol, never a line number. `Verified` is the date the row was
last checked against the tree — not when it was opened.

Created 2026-11-20 from `CHANGELOG.md` *Known issues* (kept there as history), the open caveats on
the [debug index](../debug/README.md), and the presentation-layers spec §10.

## Open

| ID | Opened | Verified | Pri | Finding | Evidence | Effort | Acceptance |
|---|---|---|---|---|---|---|---|
| B1 | 2026-09-04 | 2026-11-20 | P2 | A text edit committed on blur swallows the next click: the commit re-renders `#form`, detaching the control clicked right after typing ([real-binary finding 3](../debug/2026-09-04-real-binary-findings.md)) | `web/src/render.ts` `render()`; `tests/e2e/first-run.spec.ts` explicit `blur()` workaround | M | Typing then clicking another control applies both intents without an explicit `blur()` in the e2e test |
| B2 | 2026-09-05 | 2026-11-20 | P3 | Two ways to unset a field: `unsetButton` (ADR 0003) still renders a `.unset` button next to the row `...` menu's Unset item, so the redesign's "label \| control \| one 30px button" row is not realized | `web/src/widgets/common.ts` `unsetButton`; `web/src/rowmenu.ts` | S | A design decision is recorded and each row exposes exactly one Unset path |
| B3 | 2026-09-05 | 2026-11-20 | P3 | `manifest.webmanifest` lists `/icons/icon-*.png`, but no `web/public/icons/` exists; installs show a generic icon | `web/public/manifest.webmanifest` `icons` | S | Every manifest icon path resolves in `dist/` |
| B4 | 2026-09-05 | 2026-11-20 | P3 | The violation chip `#issues` can never appear: it shows only when `#form .summary` is hidden, and nothing sets `summary.hidden = true` | `web/src/shell.ts` chip handler and `dismissed` check | S | The summary can be dismissed and the chip then appears and restores it |
| B5 | 2026-09-05 | 2026-11-20 | P3 | `reference/presentation.md` was not updated for the settings-template redesign (row menu, Unset consolidation) | `docs/reference/presentation.md`; `web/src/rowmenu.ts` | S | `presentation.md` describes the row menu and Unset as built |

## Pending verification

Landed, but the check needs a platform or pipeline not available locally.

| Item | Closed by | Verifies when | Fallback |
|---|---|---|---|

## Awaiting external

Blocked on a person or third party. **Not counted as open** — no amount of local work closes it.

| Item | Blocked on | Ready when |
|---|---|---|
| Upstream bill item 1: `session` Cargo feature in `confy-core` | confy PR ([`reference/upstream.md`](../reference/upstream.md) *The upstream bill*) | confy-core declares `session`; drop the CI grep for `confy_core::session` |
| Upstream bill item 2: public insertion-ordinal helper in `model::` | confy PR (same section) | helper is public; confyg's own index conversion can be removed |
| Upstream bill item 3: TOML `Delete` eats the blank line after the entry (also [phase A finding 2](../debug/2026-09-04-phase-a-hands-on-findings.md)) | confy PR (same section) | `roundtrip.rs` `a_delete_keeps_the_blank_line_that_separates_a_comment_block` passes without `#[ignore]` |

## Watching

Known, deliberately not scheduled — each with the reason it is not a defect *yet*, the **trigger**
that would promote it to Open, and the date it was last re-read.

| Item | Why not now | Trigger | Re-read |
|---|---|---|---|
| `Density` values' concrete mapping to `--row-h` / `--indent` / `--hit` ([spec §10](../spec/2026-09-04-presentation-layers-design.md)) | only the TUI host needs an explicit selection | v0.3 TUI host work starts | 2026-11-20 |
| *Preset value sets* (design §7 C10) have no carrier (spec §10) | admitting them would make the profile's write-neutral reading conditional | v0.3 design starts | 2026-11-20 |
| Redesign `PATCH.md` tells `main.ts` to call `shell.onSnapshot(snapshot)`, which `Shell` does not define | instruction was skipped; `render()` already calls `appShell.afterRender()` — no code defect | `PATCH.md` moves (Wave C) or is reused as instructions | 2026-11-20 |

## Done

| ID | Finding | Closed by |
|---|---|---|
