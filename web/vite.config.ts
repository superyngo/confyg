import { createReadStream, existsSync } from "node:fs";
import { cp } from "node:fs/promises";
import { extname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import type { Plugin } from "vite";
import { defineConfig } from "vitest/config";

/**
 * Make the one URL `boundary.ts` imports resolvable.
 *
 * `wasm-pack build crates/confyg-ffi --target web` writes its glue outside this package, and
 * `boundary.ts` loads it through a non-literal specifier so a fresh checkout typechecks and
 * builds before the WASM exists. What the browser is left holding is
 * `../../crates/confyg-ffi/pkg/confyg_ffi.js`, which resolves to `/crates/confyg-ffi/pkg/...`
 * from `/src/boundary.ts` and from `/assets/index-*.js` alike — one URL that neither the dev
 * server nor `dist/` served, so the built host loaded no core at all. This makes that URL real
 * in both, and changes nothing else: the glue is copied, never bundled.
 */
function wasmPkg(): Plugin {
  const dir = fileURLToPath(new URL("../crates/confyg-ffi/pkg", import.meta.url));
  const base = "/crates/confyg-ffi/pkg/";
  const type: Record<string, string> = { ".js": "text/javascript", ".wasm": "application/wasm" };
  return {
    name: "confyg:wasm-pkg",
    configureServer(server) {
      server.middlewares.use((req, res, next) => {
        const url = req.url?.split("?")[0] ?? "";
        if (!url.startsWith(base)) return next();
        const file = join(dir, url.slice(base.length));
        if (!file.startsWith(dir) || !existsSync(file)) return next();
        res.setHeader("Content-Type", type[extname(file)] ?? "application/octet-stream");
        createReadStream(file).pipe(res);
      });
    },
    // Absent WASM is not a build failure: CI's `web` job builds the bundle without it, and
    // the missing glue surfaces as the load error it is rather than as a broken build.
    async writeBundle(options) {
      if (!existsSync(dir)) return;
      await cp(dir, resolve(options.dir ?? "dist", "crates/confyg-ffi/pkg"), { recursive: true });
    },
  };
}

export default defineConfig({
  plugins: [wasmPkg()],
  build: { target: "es2022", outDir: "dist" },
  test: {
    globals: true,
    environment: "jsdom",
    include: ["src/**/*.test.ts"],
  },
});
