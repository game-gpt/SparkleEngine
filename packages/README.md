# Spark npm 包

| 包 | 说明 |
|----|------|
| `spark-engine` | TS 元包：探测平台并加载 `spark-<platform>` |
| `spark-unknown-wasm32` | Wasm 平台包（本 workspace 成员） |
| `spark-win32-*` / `spark-darwin-*` / `spark-linux-*` | 桌面平台包（**不**进 npm workspaces，避免错误 os/cpu 安装失败；单独 `npm publish`） |

原生 `.node` 由 `projects/bindings/spark-napi`（Cargo feature `node`）构建后拷入对应平台包。

Wasm `spark_engine_bg.wasm` 由 `projects/bindings/spark-wasm` 构建后经 `spark-unknown-wasm32` 的 `npm run copy:wasm` 拷入。
