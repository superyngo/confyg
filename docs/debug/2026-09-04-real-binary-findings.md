# Real-binary check findings

Status: Resolved — findings 1 and 2 fixed, finding 3 open

What design §11 item 5 found when the flow was finally run on the actual web build: **open →
add a Repeat item → set values → Unset one → save**, against schemastore's real `.eslintrc`
Schema. Three defects, none of which the 23 Rust test binaries or the 37 jsdom tests could see,
because the jsdom suite renders a captured `SetterSnapshot` and therefore never loads the core,
never applies a stylesheet, and never emits a byte.

The two hazards Task 20 also asked to be reproduced by hand — D1 and D7 — behaved correctly.
That result is recorded at the end.

## Repro

```
wasm-pack build crates/confyg-ffi --target web
npm run test:e2e
```

`tests/e2e/first-run.spec.ts` is the flow. Findings 1 and 2 are what it hit before it could get
to an assertion; finding 3 is why it commits a text edit with an explicit `blur()`.

## 1. The built host loaded no core at all

`boundary.ts` imports the `wasm-pack` glue through a non-literal specifier so a fresh checkout
typechecks and builds before the WASM exists (which is correct and stays). What the browser was
left holding is `../../crates/confyg-ffi/pkg/confyg_ffi.js`, and that URL resolves to
`/crates/confyg-ffi/pkg/confyg_ffi.js` from `/src/boundary.ts` and from `/assets/index-*.js`
alike — a path neither the dev server nor `dist/` served:

```
response: 404 http://localhost:4173/crates/confyg-ffi/pkg/confyg_ffi.js
pageerror: Failed to fetch dynamically imported module
#form innerHTML: (empty)
```

The dev server failed identically, so the renderer had never once talked to the core in a
browser. An empty `#form` is the whole symptom: no error, no notice, nothing to read.

**Fixed** in `web/vite.config.ts` by a plugin that makes that one URL real — served from the
generated directory in dev, copied beside the bundle on build, skipped when the WASM has not
been built so CI's `web` job still builds. The glue is copied, never bundled, so `boundary.ts`
is unchanged.

The e2e's first two assertions now stand in for this: an absent `overrides` with a `(0)` badge
is only renderable by a host that loaded the core.

## 2. An Absent Repeat could never be given its first item

`style.css` hid `.repeat-add` whenever a Repeat's Occupancy was `absent`:

```css
.node.repeat[data-occupancy="absent"] > .repeat-items,
.node.repeat[data-occupancy="absent"] > .repeat-add { display: none; }
```

Playwright's report is the evidence — the button exists and cannot be used:

```
locator resolved to <button type="button" class="repeat-add">Add</button>
attempting click action — element is not visible
```

An absent collection is exactly the one **Absent-parent lowering** exists to fill: `lower.rs`
puts a header-bearing fragment into the parent for precisely this case, design §11 item 2 makes
it a mandatory round-trip case in every format, and the host hid the only affordance that
reaches it. Every absent array in every Schema was unfillable.

**Fixed**: the rule no longer names `.repeat-add`. An absent Repeat still collapses its
(empty) `.repeat-items` and keeps its heading and count.

## 3. A commit-on-blur edit swallows the next click — open

A text widget commits on `change`, i.e. on blur, and every committed intent re-renders the form
from the returned snapshot via `root.replaceChildren`. So clicking any other control right after
typing does two things at once: the blur commits and re-renders, and the click lands on a button
that has just been detached. The click is lost.

Reproduced both ways, same build:

| Sequence | Result |
|---|---|
| add → fill `parser` → click `processor`'s Unset → save | `parser` written, **Unset lost** — `"processor": ""` still in the bytes |
| add → click `processor`'s Unset → save | Unset applied, key gone |

It is not a test artifact: a human who types a value and then clicks Unset, Add, Remove or a
section tab loses that click, once, silently. The same FFI call driven directly in the page
applies the Unset correctly, so the core and the boundary are not involved:

```
hasProcessorAfterAdd: true, hasProcessorAfterUnset: false, notices: []
```

**Open, and deliberately not fixed here.** The fix is a decision about the renderer, not a
patch: either re-render without replacing subtrees the user is interacting with, or commit a
buffer before the re-render can steal the event. Both are presentation-layer design work with
no record behind them yet, and inventing one inside the verification task is how a 40-line fix
becomes a rewrite. `tests/e2e/first-run.spec.ts` sequences around it with an explicit `blur()`
and says so at the call site, so the day it is fixed the test does not silently stop covering
anything.

## The two hazards: both correct

**D1 — insert a key into a TOML table that already has a sub-table.** `[server]` holding `port`
and a `[server.tls]` sub-table; `set server.host "example.com"`:

```toml
[server]
host = "example.com"
port = 8080

[server.tls]
enabled = true
```

The key landed inside `[server]`, above the sub-table header, in **Schema `properties` order**
(`host` before `port`), and the blank line separating the sub-table survived. This is
`ordinal::schema_slot`'s clamp doing what *Insertion legality* describes.

**D7 — set a value in a file with a comment between every sibling.** `set beta 42`:

```toml
# leading note about alpha
alpha = 1
# a note between alpha and beta
beta = 42
# a note between beta and gamma
gamma = 3
# a trailing note
```

Every comment stayed attached to the entry it was written above; only the value changed.
Deleting the same key leaves its leading comment and the extra blank line that upstream bill
item 3 owns, which is the already-`#[ignore]`d
`a_delete_keeps_the_blank_line_that_separates_a_comment_block`, not a new finding.
