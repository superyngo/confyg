// The real-binary check's runner (design §11 item 5). At the repo root rather than in `web/`,
// because the thing under test spans both: the Rust core compiled to WASM and the renderer
// that loads it.
//
// `webServer` builds the bundle and serves it with `vite preview` — the built artifact, never
// a dev server, which is the whole point of item 5. `wasm-pack build crates/confyg-ffi
// --target web` is a *prior* step (CI's `e2e` job, or by hand): building the WASM here would
// spend the readiness window on a Rust compile, and a stale `pkg/` would fail the run with a
// 404 rather than a timeout.
import { defineConfig, devices } from "@playwright/test";

// `vite preview` binds `localhost`, which resolves to `::1` here — `127.0.0.1` is refused.
const PORT = 4173;
export const BUILD_URL = `http://localhost:${PORT}/`;

export default defineConfig({
  testDir: "tests/e2e",
  fullyParallel: false,
  forbidOnly: !!process.env.CI,
  retries: 0,
  reporter: process.env.CI ? "list" : "line",
  use: { ...devices["Desktop Chrome"], baseURL: BUILD_URL },
  projects: [{ name: "chromium", use: { ...devices["Desktop Chrome"] } }],
  webServer: {
    // Both halves run from the repo root: `-w web` is a workspace filter, so it only works
    // there, and `--outDir web/dist` points the preview server at what the build just wrote.
    command: `npm run build -w web && npx vite preview --port ${PORT} --strictPort --outDir web/dist`,
    url: BUILD_URL,
    reuseExistingServer: !process.env.CI,
    timeout: 120_000,
  },
});
