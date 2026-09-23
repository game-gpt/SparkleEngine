# Spark Engine

Spark 是一套用 Rust 编写的游戏引擎库：ECS、固定步时间、输入、2D/3D 绘制契约、Retained Widget、脚本编译与 VM，以及 Node.js /
Wasm 宿主绑定。引擎层不携带具体玩法数据。

产品 npm 包是 `@game-gpt/sparkle-engine`，命令行入口为 `spark`。窗口事件与 GPU 提交在 `spark-renderer-wgpu`；**权威世界与帧调度在 `SparkRuntime`（`spark-engine`）**。

## 环境与构建

- Rust 工具链：仓库根 `rust-toolchain.toml`
- Node.js ≥ 18、pnpm

```bash
pnpm install
pnpm run build:ts

cargo run -p ping-pong          # 乒乓示例（SparkRuntime）
cargo run -p snake              # 贪吃蛇示例（SparkRuntime）
cargo run -p spark-studio       # 编辑器
# 或 pnpm exec spark studio

pnpm run build:napi             # 原生 .node（按需）
rustup target add wasm32-unknown-unknown
pnpm run build:wasm             # Wasm 平台袋（按需）
```

## 模块导航

| 任务                                | crate                                                                  |
|-------------------------------------|------------------------------------------------------------------------|
| 实体 / 组件 / 调度                  | `spark-ecs`                                                            |
| 固定步时钟                          | `spark-time`                                                           |
| 键鼠帧状态                          | `spark-input`                                                          |
| 绘制列表与帧上下文                  | `spark-renderer`                                                       |
| GPU 纹理描述与上传包                | `spark-texture`                                                        |
| PNG / JPEG / WebP / KTX2 / DDS 解码 | `spark-png` / `spark-jpeg` / `spark-webp` / `spark-ktx2` / `spark-dds` |
| wgpu 窗口与提交                     | `spark-renderer-wgpu`                                                  |
| Retained UI                         | `spark-widget`                                                         |
| 脚本编译与执行                      | `spark-script` → `spark-vm`                                            |
| 运行时与模组壳                      | `spark-engine`（`SparkRuntime` / `run_runtime`）                       |
| Node 绑定                           | `spark-napi`                                                           |
| Wasm ABI                            | `spark-wasm`                                                           |
| glTF 导入                           | `spark-gltf`                                                           |

各 crate 说明见 `projects/**/readme.md`。

## 运行路径（示例）

`ping-pong` 的 `main`：

```rust
use ping_pong::runtime::build_runtime;
use spark_engine::run_runtime;
use spark_renderer::WindowConfig;

run_runtime(
    WindowConfig {
        title: "ping-pong".into(),
        width: 960,
        height: 540,
        clear_color: [0.05, 0.07, 0.10, 1.0],
    },
    build_runtime(),
)?;
```

游戏通过 `NativeGamePlugin` 向 `SparkRuntime` 注册系统；状态存放在 `World` 资源与组件中。`run_runtime` 驱动帧循环并调用 `spark-renderer-wgpu` 开窗提交。

`run_game` + `impl GameHost` 为遗留路径，已废弃，禁止新游戏使用。

## 说明

- `spark-napi` 的 N-API 导出需 `--features node`；默认 feature 为空以便纯 Rust 测试。
- `spark-jit` 当前做字节码特化，不是完整机器码后端。
- Lua / Ruby 前端是语言子集，不是完整语言运行时。
