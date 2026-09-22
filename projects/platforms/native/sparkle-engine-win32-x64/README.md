# `@game-gpt/sparkle-engine-win32-x64`

`@game-gpt/sparkle-engine` 的 win32/x64 原生平台袋：预编译 N-API 插件（`.node`），无 TypeScript 入口。

## 包内容

| 字段         | 值                                   |
|--------------|--------------------------------------|
| 包名         | `@game-gpt/sparkle-engine-win32-x64` |
| `main`       | `sparkle-engine.win32-x64-msvc.node` |
| `os` / `cpu` | `win32` / `x64`                      |

宿主包通过 optionalDependencies 安装本包，再由 `loadSpark()` 加载上述 `main`。

## 在本仓库构建

在匹配的 OS/CPU 上，于 SparkEngine 仓库根执行：

```bash
node scripts/build/napi.mjs --release
```

产物写入 `projects/platforms/native/sparkle-engine-win32-x64/`。跨平台发布需在对应机器或 CI 上分别构建。

覆盖路径：环境变量 `SPARK_NATIVE_NODE`（由宿主包解析）。

浏览器 / Wasm 使用 `@game-gpt/sparkle-engine-unknown-wasm32`，不是本包。

## 许可证

Apache-2.0
