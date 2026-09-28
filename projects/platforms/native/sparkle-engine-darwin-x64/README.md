# `@game-gpt/sparkle-engine-darwin-x64`

Native platform bundle for `@game-gpt/sparkle-engine` on darwin/x64: a prebuilt N-API plugin (`.node`), no TypeScript entry.

## Package contents

| Field | Value |
|-------|-------|
| Package | `@game-gpt/sparkle-engine-darwin-x64` |
| `main` | `sparkle-engine.darwin-x64.node` |
| `os` / `cpu` | `darwin` / `x64` |

The host package installs this via optionalDependencies; `loadSpark()` loads the `main` binary above.

## Build in this repo

On a matching OS/CPU, from the SparkEngine repo root:

```bash
node scripts/build/napi.mjs --release
```

Output lands in `projects/platforms/native/sparkle-engine-darwin-x64/`. Cross-platform releases must be built on each target machine or in CI.

Override path: environment variable `SPARK_NATIVE_NODE` (resolved by the host package).

For browser / Wasm use `@game-gpt/sparkle-engine-unknown-wasm32`, not this package.

## License

Apache-2.0
