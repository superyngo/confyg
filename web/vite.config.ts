import { defineConfig } from "vitest/config";

export default defineConfig({
  // `wasm-pack build crates/confyg-ffi --target web` writes its glue here, so the
  // renderer imports the real boundary rather than a hand-written stub.
  build: { target: "es2022", outDir: "dist" },
  test: {
    globals: true,
    environment: "jsdom",
    include: ["src/**/*.test.ts"],
  },
});
