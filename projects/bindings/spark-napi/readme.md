# `spark-napi`

Build-helper npm metadata next to the Rust crate `spark-napi` (`private: true`), not a package end users install directly.

Published surfaces:

- `@game-gpt/sparkle-engine` (JS API + CLI `spark`)
- `@game-gpt/sparkle-engine-<os-cpu>` (`.node` platform bundles)

## Build

From the SparkEngine repo root:

```bash
node scripts/build/napi.mjs --release
# or
pnpm run build:napi
```

Equivalent to `cargo build -p spark-napi --features node`, then installing artifacts into `projects/platforms/native/sparkle-engine-<short>/`.

This package's `package.json` `scripts.build` forwards to the script above; the `napi` field lists triples for toolchain reference.

## Related

- Rust API / features: `projects/bindings/spark-napi/readme.md`
- Host load logic: `projects/hosts/sparkle-engine/README.md`
