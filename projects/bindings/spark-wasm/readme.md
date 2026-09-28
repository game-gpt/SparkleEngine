# spark-wasm

`wasm32-unknown-unknown` cdylib，导出 C ABI 供平台包加载。

```bash
rustup target add wasm32-unknown-unknown
cargo build -p spark-wasm --target wasm32-unknown-unknown --release
pnpm run build:wasm
```

已导出：`spark_vec2_length`、`spark_version_code`。Rust 侧还有 `SparkWasmHost`。平台包名常量 `NPM_PLATFORM_PACKAGE`（
`@game-gpt/sparkle-engine-unknown-wasm32`）。改 ABI 后需同步 TypeScript 声明。

```bash
cargo test -p spark-wasm
```
