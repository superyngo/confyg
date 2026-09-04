# confyg

A schema-driven **configurator** for structured configuration files — JSON Schema plus a
TOML / JSON(C) / YAML file, rendered as a guided form so a config gets filled in correctly.

confyg is not a text editor and not a tree editor. It renders the *schema* as the form and
treats the file as an overlay of values that have been set. You pick from menus, add a new
config group with its defaults prefilled, add array entries within the bounds the schema
declares — and a wrong value warns rather than blocks.

Sibling project to [confy](https://github.com/superyngo/confy), whose lossless CST document
model and JSON Schema subsystem confyg reuses.

## Status

v0.1. The Rust core and the web host are implemented: a `Schema -> Form IR` compiler, a session
that lowers **Setter intents** onto a lossless CST, a WASM boundary, and a renderer.
[`docs/reference/`](docs/reference/README.md) describes current behavior;
[`CONTEXT.md`](CONTEXT.md) is the documentation index.

```sh
cargo test --workspace                                   # the core
npm ci && npm run typecheck -w web && npm test -w web    # the renderer
wasm-pack build crates/confyg-ffi --target web           # the boundary
npm run build -w web && npm run test:e2e                 # the flow, on the real build
```

Try it from a terminal without a browser:

```sh
cargo run -p confyg-session --example try -- <schema.json> <config.toml>
```

## Documentation

[`CONTEXT.md`](CONTEXT.md) is the single entry point. Start with
[`docs/reference/glossary.md`](docs/reference/glossary.md).

## License

MIT
