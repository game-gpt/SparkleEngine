# Spark Engine

通用 **2D 游戏元引擎**（Rust）。提供 ECS、时间、事件与应用壳等基础设施，**不包含**任何具体游戏玩法。

呈现目标为现代 GPU API（wgpu：DX12 / Vulkan / Metal / WebGL2 方向）。**不是**第三方引擎的 fork。

## 开发构建

工具链见 `rust-toolchain.toml`。

```bash
cargo run -p spark-app
```

## Crate 一览（当前脚手架）

| Crate | 职责 |
|-------|------|
| `spark-core` | 基础类型与错误 |
| `spark-ecs` | ECS 世界占位 |
| `spark-time` | 时间步占位 |
| `spark-event` | 事件总线占位 |
| `spark-input` | 键鼠输入状态 |
| `spark-shader` | WGSL 内建着色器与模块装载 |
| `spark-font` | 字体装载、字形栅格化与 CPU 图集 |
| `spark-render` | 窗口与 wgpu 清屏循环（消费 shader / font） |
| `spark-audio` | 混音与程序化短音框架 |
| `spark-gc` | 脚本堆与标记–清扫 GC |
| `spark-vm` | 栈式字节码虚拟机 |
| `spark-jit` | 热路径字节码特化（基线 JIT） |
| `spark-script` | 脚本引擎（Oaks `oak-core` 源码前端 → VM） |
| `spark-app` | 应用壳演示入口 |

