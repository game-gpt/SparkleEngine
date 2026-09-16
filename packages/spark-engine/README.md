# `spark-engine`（npm）

Spark 元引擎的 **TypeScript 入口包**。原生侧按宿主平台解析 `spark-<short>` **二进制袋**（`main` 即 `spark.<triple>.node`），与 vmz-framework 的 `vmz-<short>` 布局一致；**不是**再包一层 TypeScript 平台包。

## 安装

```bash
npm install spark-engine
```

`optionalDependencies` 会按 `os` / `cpu` 拉取匹配的原生包。浏览器 / WASI 请显式依赖 `spark-unknown-wasm32`。

## 构建原生插件

在仓库根（Rust workspace）：

```bash
node scripts/build/napi.mjs
# 或 release：
node scripts/build/napi.mjs --release
```

脚本会 `cargo build -p spark-napi --features node`，并把产物写入 `packages/spark-<short>/spark.<triple>.node`。

也可用环境变量覆盖路径：`SPARK_NATIVE_NODE=/path/to/spark.….node`。

## 用法

```js
const { loadSpark, resolveNativePath } = require("spark-engine");

const host = loadSpark();
console.log(host.info());
console.log(host.vec2Length(3, 4));
```
