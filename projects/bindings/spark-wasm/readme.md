# spark-wasm

`wasm32-unknown-unknown` cdylib exporting a C ABI for platform packages to load.

```bash
rustup target add wasm32-unknown-unknown
cargo build -p spark-wasm --target wasm32-unknown-unknown --release
pnpm run build:wasm
```

Exported: `spark_vec2_length`, `spark_version_code`. Rust side also has `SparkWasmHost`. Platform package constant `NPM_PLATFORM_PACKAGE` (`@game-gpt/sparkle-engine-unknown-wasm32`). Keep TypeScript declarations in sync when changing the ABI.

```bash
cargo test -p spark-wasm
```
