// The real-binary check (design §11 item 5): open → add a Repeat item → set values →
// Unset one → save, driven through the built web host against a real published Schema.
//
// Why this file exists at all: green unit tests are not evidence. The jsdom suite renders a
// captured `SetterSnapshot`, so it cannot see whether the bundle loads the WASM core, whether
// an affordance the core needs is reachable, or what bytes leave the host. Writing this test
// found both — a glue URL nothing served, and an `Add` button CSS hid on exactly the absent
// collection the core's Absent-parent lowering exists to fill.
//
// Nodes are addressed by `data-path`, not by label: a Field's name is a `<span
// class="field-label">` and is not associated with its control, so `getByLabel` finds nothing.
// `data-path` carries `confyg_form::search::path_text`'s rendering, which the reference docs
// name as the string a host addresses a node by.
import { expect, test } from "@playwright/test";

/** The Task 6 fixture: schemastore's real `.eslintrc` Schema, 55 KB of it. */
const SCHEMA = "crates/confyg-form/tests/fixtures/eslintrc.json";

/**
 * `overrides` is the Repeat under test. In the eslintrc Schema it is a depth-1 array of
 * objects, so the Partition leaves it `loose` — always on screen, no navigation needed — and
 * it is **Absent** in this document, which makes the flow exercise the absent-parent path.
 *
 * `parser` and `processor` are the two entry members that project as real `text` Fields;
 * `files`, `extends` and `excludedFiles` are `oneOf`, which v0.1 projects as Unknown.
 */
// Authored as text, not as an object: byte-identical preservation of the untouched region is
// one of the things this test checks, and `JSON.stringify` would decide the layout instead.
const DOC = `{
  "root": true,
  "plugins": ["react"],
  "extends": "eslint:recommended",
  "rules": {
    "comma-dangle": 2
  }
}
`;

test.beforeEach(async ({ page }) => {
  // Take the non-Chromium branch `host-io.ts` already documents: with no
  // `showSaveFilePicker`, Save downloads, and a download is bytes this test can read. The
  // alternative — driving Chromium's native picker — is not automatable, and shimming
  // `FileSystemFileHandle` would test the shim. See `docs/reference/presentation.md`.
  await page.addInitScript(() => {
    // @ts-expect-error deleting an optional platform API is the point
    delete window.showSaveFilePicker;
  });
  await page.goto("/");
});

// A bare page load renders the built-in sample. This is the assertion that would have
// caught finding 1 on its own: with the glue unresolvable the boundary never loads, so
// there is no snapshot, no nodes, and an empty pane — which every unit test above this
// one reads as fine, because it renders a captured snapshot instead of booting.
test("a bare load renders the sample, not an empty pane", async ({ page }) => {
  await expect(page.locator(".node.field").first()).toBeVisible();
  expect(await page.locator(".node.field").count()).toBeGreaterThan(0);
  // The sample's deliberate edges, straight from the fixtures the Rust suite pins:
  // `port = 99999` is out of range, and `colour` is a key the Schema never heard of.
  await expect(page.locator(".control.invalid")).toHaveCount(1);
  await expect(page.locator(".node.unknown")).toHaveCount(1);
  // The latch: no backing file yet.
  await expect(page.locator("body.sample")).toHaveCount(1);
});

test("open → add a Repeat item → set values → Unset one → save", async ({ page }) => {
  // The session never fetches: the host resolves the Schema and dispatches the bytes back.
  // `main.ts` builds a detached `<input type=file>` for it, so this is a filechooser, not a
  // locator.
  const chooser = page.waitForEvent("filechooser");
  await page.getByRole("button", { name: "Schema" }).click();
  await (await chooser).setFiles(SCHEMA);

  await page.locator("#open").setInputFiles({
    name: ".eslintrc.json",
    mimeType: "application/json",
    buffer: Buffer.from(DOC),
  });

  // Rendering at all is the first assertion: an empty `#form` is what a core that failed to
  // load looks like.
  const overrides = page.locator('.repeat[data-path="overrides"]');
  // `:scope >` throughout: an eslintrc override entry carries its own nested `overrides` and
  // `plugins` Repeats, so a descendant selector matches three counts once an entry exists.
  const count = overrides.locator(":scope > .node-heading > .repeat-count");
  await expect(overrides).toHaveAttribute("data-occupancy", "absent");
  await expect(count).toHaveText("(0)");

  await overrides.locator(":scope > .repeat-add").click();
  await expect(count).toHaveText("(1)");
  await expect(overrides).toHaveAttribute("data-occupancy", "populated");

  // Set values. A text widget commits on `change`, i.e. on blur, and every committed intent
  // re-renders the form from the returned snapshot. `blur()` makes that commit happen here
  // rather than inside the *next* click, whose own blur would otherwise replace the button
  // mid-click and lose it — see
  // `docs/debug/2026-09-04-real-binary-findings.md` finding 3, which this sequencing sidesteps
  // deliberately rather than papers over.
  const parser = page.locator('.field[data-path="overrides[0].parser"] input');
  await parser.fill("babel-eslint");
  await parser.blur();
  await expect(page.locator('.field[data-path="overrides[0].parser"] input')).toHaveValue(
    "babel-eslint",
  );

  // Unset one: the Add fragment wrote a placeholder for every member, and Unset deletes the
  // key rather than writing an empty value (design §11 item 3). The affordance disables
  // itself once the value is Absent, which is the host reading the new snapshot back.
  await page.locator('.field[data-path="overrides[0].processor"] .unset').click();
  await expect(page.locator('.field[data-path="overrides[0].processor"] .unset')).toBeDisabled();

  const download = page.waitForEvent("download");
  await page.getByRole("button", { name: "Save" }).click();
  const saved = await readDownload(await download);

  expect(saved).toContain("babel-eslint");
  // The Unset member is gone from the bytes, placeholder and all. eslintrc has no key spelled
  // `placeholder`, so the plan's literal string would have passed vacuously; the member this
  // flow actually unset is the honest assertion.
  expect(saved).not.toContain("processor");
  // Everything untouched is still byte-identical (Minimal write).
  expect(saved).toContain('"comma-dangle": 2');
  expect(saved).toContain('"plugins": ["react"]');
});

async function readDownload(download: {
  createReadStream(): Promise<NodeJS.ReadableStream>;
}): Promise<string> {
  const stream = await download.createReadStream();
  let text = "";
  for await (const chunk of stream) text += chunk;
  return text;
}
