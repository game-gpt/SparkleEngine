# `@game-gpt/sparkle-engine-unknown-wasm32`

Spark Engine 的 Wasm 平台袋：TypeScript 加载器 + `spark_engine_bg.wasm`（由 Rust crate `spark-wasm` 构建拷贝而来）。

## 安装

```bash
npm install @game-gpt/sparkle-engine-unknown-wasm32
```

ESM 包（`"type": "module"`）。Node ≥ 18；浏览器需能 `fetch` / 实例化 Wasm。

## 用法

```js
import { loadSpark, rustTarget, platformPackage } from "@game-gpt/sparkle-engine-unknown-wasm32";

const spark = await loadSpark();
console.log(spark.info());
console.log(spark.vec2Length(3, 4));
```

可选：

```js
await loadSpark({ wasmUrl: new URL("./spark_engine_bg.wasm", import.meta.url) });
// 或传入已编译的 WebAssembly.Module
await loadSpark({ module });
```

若 `.wasm` 尚未拷贝到位，加载器会回退到纯 JS 的 `Math.hypot` 实现，方便 TS 联调；正式环境请先构建 Wasm。

## 在本仓库构建

```bash
rustup target add wasm32-unknown-unknown
pnpm --filter @game-gpt/sparkle-engine-unknown-wasm32 run build:wasm
# 或仓库根：pnpm run build:wasm
```

`build:wasm` 会 `cargo build -p spark-wasm --target wasm32-unknown-unknown --release`，再把产物拷成包根的
`spark_engine_bg.wasm`。

常量：`rustTarget === "wasm32-unknown-unknown"`，`platformPackage === "spark-unknown-wasm32"`。

原生 Node `.node` 请用 `@game-gpt/sparkle-engine` 的 `loadSpark`，不要混用本包入口。

## 许可证

Apache-2.0
