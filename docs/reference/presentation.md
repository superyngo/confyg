# Presentation

How the **Form IR** becomes a screen, as `web/` does it today. Canonical for the *host
contract*; the layer model and its reasoning live in
[the presentation record](../spec/2026-09-04-presentation-layers-design.md).

The rule the whole layer is built to keep: **no presentation input may change a written byte**
(ADR 0004). `confyg-session/tests/write_neutrality.rs` runs eleven Annotations × four Host
profiles × three formats and asserts identical bytes, and CI gives it its own job.

## The host makes no form decisions

Every decision arrives in the `SetterSnapshot`. The renderer does not clamp Widgets, derive
menu choices, validate literals, or score a search — it mounts what the IR names.
`web/src/render.ts` `render(snapshot, root, ctx)` is the entire entry point.

A host addresses nodes by Path, rendered as `data-path` (`servers[0].host`), which is the same
string `confyg_form::search::path_text` produces. Field labels are `<span class="field-label">`
and are *not* associated with their controls, so `data-path` is the only reliable handle — the
e2e suite uses it for exactly that reason.

## Partition

`web/src/partition.ts` `partition(ir)` cuts the tree into screens. The closed set is four;
v0.1 implements two, and `tabs` and `wizard` clamp to `sections` so a profile written today
stays readable.

| Kind | When | Layout |
|---|---|---|
| `scroll` | Fewer than three depth-1 **Groups** | One page |
| `sections` | Three or more | A section list beside one section's fields |

Only depth-1 **Groups** become sections. Everything else at depth 1 — a Field, a **Repeat**, an
`unknown` — is `loose`, and loose nodes render above the section list, always on screen. That is
why a depth-1 array like eslintrc's `overrides` needs no navigation to reach.

Navigation is free: every section is reachable at any time, and nothing is withheld or disabled
for a missing or invalid value (ADR 0004 decision 5). `reveal(path)` moves to the section
holding a node and marks the node with a class — never `focus()`, which would fight a screen
reader by pulling focus somewhere the user did not ask for.

## Widgets and the three Presence states

`web/src/widgets/index.ts` `mount(field, ctx)` looks the control up in a
`Record<Widget, Mount>` keyed by the closed vocabulary, so an unmapped Widget is a build error.
Every Widget reaches all three **Presence** states, by one of two routes (ADR 0003):

- A control whose own value space can hold "inherit" gets an **inherited option** — a `tristate`
  boolean is therefore three-state (`inherited` / `true` / `false`), never a bare checkbox.
- A control that cannot express absence — the whole text family, where an empty string is a
  value — keeps a separate **Unset** affordance, disabled once the value is already Absent.

An Absent field shows its default as **Ghost text**, or *not set* when it has none. A clamped
Widget renders a **degradation notice** naming what the Schema asked for; the clamp itself
already happened in the core. A **Locked** or `readOnly` field mounts as display only whatever
its Widget.

`ctx.set` takes a **JSON literal**: a string field's text is quoted on the way out, so typing
`123` into a string stays `"123"`. `rawText` is the one exception by definition — the Raw
literal fallback hands over bytes as authored.

## Repeat cards

`web/src/repeat.ts` `renderRepeat(node, depth, ctx)`: a card per object entry, a row per scalar
one — a card around a single text field is chrome with nothing in it. The count badge reads
`(3/5)` when the Schema set a ceiling and `(3)` when it did not. A card's `data-index` is an
**entry** index, which is what `removeRepeatItem` takes.

The two bounds gates disable `Add` at `maxItems` and `Remove` at `minItems`, using the same two
comparisons `lower.rs` refuses on, so the host never offers an intent the core would decline.

`Add` is deliberately *not* hidden on an **Absent** collection: that is precisely the collection
**Absent-parent lowering** exists to fill. Hiding it made every absent array in every Schema
unfillable, which is
[finding 2](../debug/2026-09-04-real-binary-findings.md) of the real-binary check.

## The violation summary

`web/src/summary.ts` `renderSummary(summary, jump)` sits above the form in both Partitions: a
violation the user cannot see is a violation they cannot fix, whichever screen holds the field.
Items jump to their node through `reveal`. Counts are a projection of the existing Violation set
grouped by Partition, not a new computation.

When the Schema could not be compiled at all, the summary says **validation unavailable** and
never *no problems* (D8).

## Form search

`confyg_form::search::search(&root, query)` ranks the hits; `web/src/search.ts` only decides
which section a hit belongs to. The host never scores, filters or re-sorts — two host-side
matchers guarantee the Web and the TUI answer the same query differently.

Three axes are scored — title, description, Path — and the best one wins, so a title hit ranks a
node whose Path says nothing and the reverse. An empty query returns nothing rather than
everything: a result list containing the whole tree is not a result. Ties break on Path, so the
same query never reorders its own results between calls.

This is not the **Option filter**, which filters choices *inside* a Widget and treats an empty
needle as "everything matches". They share no code and no term.

### A subtree matches as a whole, and that ships

Because the Path is an axis and a descendant's Path contains its ancestor's key, a query naming
a container matches the container *and every node beneath it*, at the same score:

```
find server
  131  servers            servers
  131  servers[0]         Server
  131  servers[0].host    host
  131  servers[0].weight  weight
  ...                     (10 rows, one score)
```

**Decision for v0.1: ship it.** The Path tiebreak already orders them parent-first, so the list
reads as "the container, then what is in it" rather than as noise. Collapsing subtree hits needs
a dominant-ancestor rule, and any such rule also suppresses a descendant that matched on its own
*title* or *description* — which is the case the three axes exist to catch. Ranking a container
above its descendants instead of tying them is the cheaper improvement, and it belongs with the
scoring work in v0.2, not with a suppression pass bolted on now.

## Appearance and Lexicon

Appearance is CSS custom properties in `web/src/tokens.css`, `oklch` throughout, switched by a
`data-theme` attribute on `<html>` with two values, `dark` and `light`. Nothing in the token
layer can reach a written byte, which is the point.

Chassis strings — buttons, notices, widget names — are a flat catalog under `i18n/{en,zh-TW}.json`
with a `form.*` prefix, applied by `data-i18n` attributes and `t`/`tArgs`. Schema content
strings (`title`, `description`) belong to the Schema and are versioned with it; the two catalogs
are not merged.

## Host I/O

The host owns what the session refuses to: files, the Schema fetch, and the theme.

`saveText(text, fmt, name, handle)` writes back in one of three ways, in order: an already-open
`FileSystemFileHandle` is written in place — an open handle never needs a picker, because a
picker only chooses a *new* destination; otherwise `showSaveFilePicker` offers one; otherwise the
text downloads. The last branch is not an error path, it is what every non-Chromium browser does.

**Which branch is machine-tested, stated plainly.** The e2e suite deletes
`window.showSaveFilePicker` before the page loads, so Save takes the download branch and
`page.waitForEvent("download")` yields the real emitted bytes. **The download path is therefore
the tested one, and the in-place File System Access write is hand-verified only** — the same
honesty as the `#[ignore]`d byte test in [`crates.md`](crates.md). Driving Chromium's native
picker is not automatable, and shimming `FileSystemFileHandle` would test the shim.

The session never fetches: a `SchemaFetchRequest` in a snapshot is resolved by the host and
dispatched back as `loadSchema`.

## A known defect

A text widget commits on `change`, i.e. on blur, and every committed intent re-renders the form
by replacing `#form`'s children. Clicking another control immediately after typing therefore
commits, re-renders, and loses that click — once, silently.
[Finding 3](../debug/2026-09-04-real-binary-findings.md) has the repro and why the fix is a
renderer decision rather than a patch. `tests/e2e/first-run.spec.ts` sequences around it with an
explicit `blur()` and says so at the call site.
