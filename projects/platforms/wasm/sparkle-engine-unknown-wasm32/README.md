# `@game-gpt/sparkle-engine-unknown-wasm32`

Wasm platform bundle for Spark Engine: TypeScript loader + `spark_engine_bg.wasm` (built from Rust crate `spark-wasm` and copied in).

## Install

```bash
npm install @game-gpt/sparkle-engine-unknown-wasm32
```

ESM package (`"type": "module"`). Node ≥ 18; browsers must be able to `fetch` / instantiate Wasm.

## Usage

```js
import {loadSpark, rustTarget, platformPackage} from "@game-gpt/sparkle-engine-unknown-wasm32";

const spark = await loadSpark();
console.log(spark.info());
console.log(spark.vec2Length(3, 4));
```

Optional:

```js
await loadSpark({wasmUrl: new URL("./spark_engine_bg.wasm", import.meta.url)});
// or pass a compiled WebAssembly.Module
await loadSpark({module});
```

If `.wasm` is not copied yet, the loader falls back to a pure JS `Math.hypot` implementation for TS integration; build Wasm for production.

## Build in this repo

```bash
rustup target add wasm32-unknown-unknown
pnpm --filter @game-gpt/sparkle-engine-unknown-wasm32 run build:wasm
# or from repo root: pnpm run build:wasm
```

`build:wasm` runs `cargo build -p spark-wasm --target wasm32-unknown-unknown --release`, then copies the artifact to package-root `spark_engine_bg.wasm`.

Constants: `rustTarget === "wasm32-unknown-unknown"`, `platformPackage === "spark-unknown-wasm32"`.

For native Node `.node`, use `loadSpark` from `@game-gpt/sparkle-engine`; do not mix entry points with this package.

## License

Apache-2.0
