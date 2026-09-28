# `@game-gpt/sparkle-engine`

Spark Engine 的 npm 入口：加载当前平台的原生 N-API 插件（`.node`），并提供全仓库唯一 CLI `spark`。

## 安装

```bash
npm install @game-gpt/sparkle-engine
# 或 pnpm add @game-gpt/sparkle-engine
```

Node.js ≥ 18。对应平台的 optionalDependency（如 `@game-gpt/sparkle-engine-win32-x64`）会随安装拉下来；本机没有匹配平台时，需要自行构建原生袋。

在本仓库开发：

```bash
pnpm install
pnpm run build:ts
pnpm run build:napi    # 写入 projects/platforms/native/sparkle-engine-<short>/
```

## JavaScript API

```js
const { loadSpark, currentNativePackage, listPlatformPackages } = require("@game-gpt/sparkle-engine");

console.log(currentNativePackage());
// → "@game-gpt/sparkle-engine-win32-x64" 等

const spark = loadSpark();
console.log(spark.info());
// → { name, version, npmPackage }

console.log(spark.vec2Length(3, 4)); // 5

const id = spark.loadBytes("/path/to/assets", "a.txt");
console.log(spark.assetLen(id));
```

- `loadSpark()`：`require` 当前平台 `.node`，构造 `JsSparkHost`，缓存单例。
- 强制 Wasm：不要走 `loadSpark`，改用 `@game-gpt/sparkle-engine-unknown-wasm32` 的 `loadSpark`。
- 覆盖二进制路径：环境变量 `SPARK_NATIVE_NODE` 指向某个 `.node` 文件。

## CLI `spark`

```bash
pnpm exec spark info
pnpm exec spark run [--cwd <game-project>]
pnpm exec spark studio [--cwd <game-project>] [--play] [--safe-mode]
```

| 命令     | 行为                                                                                        |
|----------|---------------------------------------------------------------------------------------------|
| `info`   | 加载原生绑定并打印 `info()` JSON                                                            |
| `run`    | 读项目 `package.json` 的 `spark.runTarget` → `cargo run -p …`；没有则 `spark-studio --play` |
| `studio` | 启动 `spark-studio` 二进制（需先 `cargo build -p spark-studio`）                            |

游戏项目目录须含 `package.json`。Studio 二进制也可通过 `SPARK_STUDIO_BIN` 指定。

## 平台包

| 包名                                      | 用途                           |
|-------------------------------------------|--------------------------------|
| `@game-gpt/sparkle-engine-win32-x64` 等   | 仅含预编译 `.node`             |
| `@game-gpt/sparkle-engine-unknown-wasm32` | 浏览器 / Wasm 加载器 + `.wasm` |

列表见 `listPlatformPackages()`。

## 许可证

Apache-2.0
