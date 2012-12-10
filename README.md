# Spark Engine

通用 **2D 游戏元引擎**（Rust）。提供 ECS、时间、事件与 Wasm / Node 派发等基础设施， **不包含**任何具体游戏玩法。

呈现目标为现代 GPU API（wgpu：DX12 / Vulkan / Metal / WebGL2 方向）。 **不是**第三方引擎的 fork。

产品 npm 面为 **`@game-gpt/sparkle-engine`**（唯一 CLI：`spark`）。

## 开发构建

工具链见 `rust-toolchain.toml`。JS 侧用 pnpm workspace（仓库根）。

```bash
pnpm install
pnpm run build:ts
node scripts/build/napi.mjs

# Studio
cargo run -p spark-studio
# 或
pnpm exec spark studio

# Wasm
rustup target add wasm32-unknown-unknown
pnpm run build:wasm
```

## 目录布局

```text
projects/
  foundation/   # 基础运行时
  graphics/     # 渲染与媒体（含 spark-widget）
  scripting/    # GC / VM / 脚本前端
  gameplay/     # 模组壳、体裁引擎、spark-studio
  bindings/     # spark-wasm / spark-napi
  hosts/        # @game-gpt/sparkle-engine（唯一 bin：spark）
  platforms/
    native/     # @game-gpt/sparkle-engine-<os-cpu> 原生袋
    wasm/       # @game-gpt/sparkle-engine-unknown-wasm32
```

**禁止**仓库根 `packages/`。引擎 crate 与 Studio **不得**再声明 npm `bin`。
