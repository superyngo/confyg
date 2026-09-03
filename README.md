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

Design phase. See [`docs/spec/`](docs/spec/README.md) for the design record and
[`CONTEXT.md`](CONTEXT.md) for the documentation index. No implementation yet.

## Documentation

[`CONTEXT.md`](CONTEXT.md) is the single entry point. Start with
[`docs/reference/glossary.md`](docs/reference/glossary.md).

## License

MIT
