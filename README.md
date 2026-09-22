# Spark Engine

Spark 是一套用 Rust 编写的游戏引擎库：ECS、固定步时间、输入、2D/3D 绘制契约、Retained Widget、脚本编译与 VM，以及 Node.js /
Wasm 宿主绑定。引擎层不携带具体玩法数据。

产品 npm 包是 `@game-gpt/sparkle-engine`，命令行入口为 `spark`。窗口事件与 GPU 提交在 `spark-renderer-wgpu`；固定步帧编排与模组壳在
`spark-engine`。

## 环境与构建

- Rust 工具链：仓库根 `rust-toolchain.toml`
- Node.js ≥ 18、pnpm

```bash
pnpm install
pnpm run build:ts

cargo run -p ping-pong          # 乒乓示例窗口
cargo run -p spark-studio       # 编辑器
# 或 pnpm exec spark studio

pnpm run build:napi             # 原生 .node（按需）
rustup target add wasm32-unknown-unknown
pnpm run build:wasm             # Wasm 平台袋（按需）
```

## 模块导航

| 任务                  | crate                       |
|-----------------------|-----------------------------|
| 实体 / 组件 / 调度    | `spark-ecs`                 |
| 固定步时钟            | `spark-time`                |
| 键鼠帧状态            | `spark-input`               |
| 绘制列表与 `GameHost` | `spark-renderer`            |
| GPU 纹理描述与上传包  | `spark-texture`             |
| 源图解码 → 上传包     | `spark-image-formats`       |
| wgpu 窗口与提交       | `spark-renderer-wgpu`       |
| Retained UI           | `spark-widget`              |
| 脚本编译与执行        | `spark-script` → `spark-vm` |
| 模组与帧循环          | `spark-engine`              |
| Node 绑定             | `spark-napi`                |
| Wasm ABI              | `spark-wasm`                |
| glTF 导入             | `spark-gltf`                |

各 crate 说明见 `projects/**/readme.md`。

## 运行路径（示例）

`ping-pong` 的 `main`：

```rust
use ping_pong::PingPongGame;
use spark_engine::run_game;
use spark_renderer::WindowConfig;

run_game(
    WindowConfig {
        title: "ping-pong".into(),
        width: 960,
        height: 540,
        clear_color: [0.05, 0.07, 0.10, 1.0],
    },
    PingPongGame::new(),
)?;
```

`PingPongGame` 实现 `GameHost`：`update` 读 `FrameCtx.input`，`draw` 往 `DrawList` 填矩形。`run_game` 负责帧循环并调用
`spark-renderer-wgpu` 开窗提交。

## 说明

- `spark-napi` 的 N-API 导出需 `--features node`；默认 feature 为空以便纯 Rust 测试。
- `spark-jit` 当前做字节码特化，不是完整机器码后端。
- Lua / Ruby 前端是语言子集，不是完整语言运行时。
