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
| `spark-geometry` | 2D 几何：向量运算、圆/线段/射线、变换与相交 |
| `spark-ecs` | ECS 世界占位 |
| `spark-time` | 时间步占位 |
| `spark-event` | 事件总线占位 |
| `spark-input` | 键鼠输入状态 |
| `spark-shader` | WGSL 内建着色器与模块装载 |
| `spark-font` | 字体装载、字形栅格化与 CPU 图集 |
| `spark-image` | 像素图装载、精灵裁切与九宫格拉伸 |
| `spark-renderer` | 2D 绘制列表 / 帧上下文 / 宿主契约（后端无关） |
| `spark-renderer-wgpu` | wgpu 窗口壳与批绘制（消费 shader / font） |
| `spark-media` | Symphonia 容器探测 / 解复用 / 音频 PCM 解码（audio·video 共用） |
| `spark-audio` | 混音、程序化短音与文件播放（消费 `spark-media`） |
| `spark-video` | 视频轨解复用与压缩包泵（消费 `spark-media`） |
| `spark-gc` | 脚本堆与标记–清扫 GC |
| `spark-vm` | 栈式字节码虚拟机 |
| `spark-jit` | 热路径字节码特化（基线 JIT） |
| `spark-script` | 脚本引擎门面（多前端 → VM） |
| `spark-script-valkyrie` | Valkyrie 前端（Oaks） |
| `spark-script-lua` | Lua 前端（Oaks） |
| `spark-script-ruby` | Ruby / RGSS 子集前端 |
| `spark-engine` | VM 之上的模组加载、钩子、数据表与资源挂载（模组跑 `spark-vm`） |
| `spark-engine-rts` | RTS 特异化：选取、指令队列、迷雾骨架 |
| `spark-engine-stg` | STG 特异化：弹幕池、发射器、判定与关卡时钟 |
| `spark-engine-rpg` | RPG 特异化：队伍、背包、属性、任务、回合序 |
| `spark-engine-platformer` | 平台跳跃：刚体、固体/单向台、土狼跳、相机 |
| `spark-engine-galgame` | Galgame/AVG：对白、选项、旗标、图层槽；默认整合 Live2D 脚本插件 |
| `spark-plugin` | 脚本插件机制：向 `spark-vm` 注册原生能力（Rust 侧请直接依赖 crate） |
| `spark-plugin-live2d` | Live2D 脚本插件：模型句柄、参数与动作（可换 Cubism 后端） |
| `spark-plugin-steam` | Steam 脚本插件：成就、统计、云文件与用户信息（可换 Steamworks 后端） |
| `spark-app` | 应用壳演示入口 |

