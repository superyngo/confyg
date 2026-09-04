# confyg settings-template redesign — patch set

Mirror of `confyg/` paths. Copy each file over the repo file of the same name.

| file | change |
| --- | --- |
| `web/index.html` | replace — new chrome (file chip, dirty dot, violation chip, gear app menu, touch back bar), manifest + service-worker hooks |
| `web/src/tokens.css` | replace — IBM Plex Sans/TC + the density/scale/layout tokens the shell writes to |
| `web/src/style.css` | replace — the whole shell: three-column master-detail, label/control/menu field grid, entry column + drawer, coarse-pointer and narrow-width adaptation |
| `web/src/shell.ts` | new — chrome state: dirty, violation chip, app menu, density/text-size/theme/language prefs, entry drawer, install prompt, width class |
| `web/src/rowmenu.ts` | new — the per-field `...` menu (unset / copy value / raw literal / constraints / schema info) and the hover tooltip |
| `web/src/render.ts` | replace — field rows become a 3-track grid with a menu button and a tooltip; a Repeat-only section gets an entry column; Unknown keys fold to the page foot |
| `web/src/partition.ts` | replace — `SECTIONS_FLOOR` 3 -> 2 and a `repeatOnly` flag per section so the layout knows to draw an entry column |
| `web/manifest.webmanifest` | new — installable app, file handlers for .toml/.json/.yaml, share target |
| `web/sw.js` | new — app-shell precache, network-first for the wasm bundle |
| `i18n/*.additions.json` | merge into `i18n/en.json` and `i18n/zh-TW.json` |

## Wiring in `main.ts`

Three additions, no restructuring:

```ts
import { shell } from "./shell.js";      // after the other imports

shell.init();                            // next to initTheme() / applyStaticI18n()

// in show(): the snapshot is the only thing that knows the current violations
shell.onSnapshot(snapshot);              // after render(snapshot, root, session)

// in the save handler, after saveText(...)
shell.markClean(name, fmt);
```

and mark dirty from the one place every write goes through — `session.set` / `unset` /
`addItem` / `removeItem` / `toggleGroup` all call `show()`, so `shell.markDirty()` at the
top of `show()` is enough (guard it with a flag so the initial sample render stays clean).

Register the worker at the end of `main.ts`:

```ts
if ("serviceWorker" in navigator) navigator.serviceWorker.register("/sw.js");
```

## One thing the core still owes the host

The `...` menu and the hover tooltip want the field's Schema node verbatim
(`{ "type": "integer", "minimum": 1, "maximum": 65535 }`). `NodeMeta` does not carry it,
so `rowmenu.ts` derives what it can from `meta` + `widget` + `presence` and says so.
Adding `meta.schema: string` (the compiled subschema, serialised) in `confyg-form` would
make both exact; `schemaText()` is the single place to change.

## Deliberate design decisions

- Every integer control is gated on input: non-digits never reach the CST and the Schema
  ceiling clamps as you type; below-minimum stays reachable so a half-typed number is not
  rewritten under the caret, and shows as a violation instead. See `guardInt` in
  `rowmenu.ts` — wire it in each numeric widget (`stepper`, `slider`).
- Application preferences (appearance, language, about, PWA) live behind the gear popover,
  never in the section list: the sidebar is for the document being edited.
- Density and text size are presentation only. They write `--row-h` / `--font-scale` and
  are persisted in `localStorage`; a coarse pointer lifts every hit area to `--hit`.
