# Spark Engine

通用 **2D 游戏元引擎**（Rust）。提供 ECS、时间、事件与 Wasm / Node 派发等基础设施， **不包含**任何具体游戏玩法。

呈现目标为现代 GPU API（wgpu：DX12 / Vulkan / Metal / WebGL2 方向）。 **不是**第三方引擎的 fork。

## 开发构建

工具链见 `rust-toolchain.toml`。

```bash
# 原生 N-API → packages/spark-<short>/spark.<triple>.node
node scripts/build/napi.mjs

# Wasm（spark-wasm）→ 拷入 npm Wasm 包
rustup target add wasm32-unknown-unknown
cargo build -p spark-wasm --target wasm32-unknown-unknown --release
npm run copy:wasm --prefix packages/spark-unknown-wasm32
```

## 目录布局

```text
projects/
  foundation/   # 基础运行时（core / diagnostics / logger / ecs / time / event / input / geometry / physics / net / asset / localization / debugger）
  graphics/     # 渲染与媒体（renderer / widget / audio / video …）
  scripting/    # GC / VM / 脚本前端
  gameplay/     # 模组壳、体裁引擎、脚本插件
  bindings/     # 派发绑定（spark-wasm / spark-napi）
```

npm 包在仓库 `packages/`（不进 Cargo members）：

```text
packages/
  spark-engine/           # TS 元包：resolveNativePath → require(.node)
  spark-win32-x64/        # 二进制袋（main = spark.win32-x64-msvc.node）
  spark-darwin-arm64/     # …
  spark-linux-x64/        # …
  spark-unknown-wasm32/   # Wasm（承接 spark-wasm 产物；可有 TS 胶水）
```

## Crate 一览（当前脚手架）

### foundation

| Crate                | 职责                                                            |
|----------------------|-----------------------------------------------------------------|
| `spark-core`         | 基础类型与错误别名                                              |
| `spark-diagnostics`  | 结构化 Error / Diagnostic / ErrorArgs（无用户最终句子）         |
| `spark-logger`       | 分级日志、可插拔 sink、结构化 `LogEvent`（`spark_event!`）      |
| `spark-geometry`     | 2D/3D 几何：向量、`Quat`/`Trs`/`Mat4`、圆/射线、变换与相交      |
| `spark-ecs`          | Archetype ECS：Entity / Component / Query / Resource / Schedule |
| `spark-time`         | 固定/可变时间步、缩放与暂停                                     |
| `spark-event`        | 事件总线占位                                                    |
| `spark-input`        | 键鼠输入状态                                                    |
| `spark-physics`      | 2D 刚体、宽窄相与步进框架                                       |
| `spark-circuit`      | 导体图、通道与可达查询框架                                      |
| `spark-net`          | 传输抽象、包序号与客户端预测框架                                |
| `spark-asset`        | 资源句柄、缓存、加载器与热重载钩子                              |
| `spark-localization` | Locale 协商、消息模型、不可变快照与诊断                         |
| `spark-animator`    | 动画剪辑、播放进度、状态机，以及骨骼姿态采样（不绘制）      |

### graphics

| Crate                 | 职责                                                               |
|-----------------------|--------------------------------------------------------------------|
| `spark-shader`        | WGSL 内建着色器与模块装载                                          |
| `spark-gltf`          | glTF 2.0 导入：骨架、剪辑与蒙皮网格（无 GPU）                      |
| `spark-font`          | 字体装载、字形栅格化与 CPU 图集                                    |
| `spark-image`         | 像素图装载、精灵裁切与九宫格拉伸                                   |
| `spark-renderer`      | 2D/3D 绘制列表、帧上下文、蒙皮网格契约（后端无关）                 |
| `spark-renderer-wgpu` | wgpu 窗口事件泵与批绘制（含蒙皮 palette；帧编排在 `spark-engine`） |
| `spark-widget`        | Retained Widget 系统：`UiRuntime` / `WidgetTree`、布局与事件骨架、主题、Overlay、motion、paint→`DrawList` |
| `spark-media`         | Symphonia 容器探测 / 解复用 / 音频 PCM 解码（audio·video 共用）    |
| `spark-audio`         | 混音、程序化短音与文件播放（消费 `spark-media`）                   |
| `spark-video`         | 视频轨解复用与压缩包泵（消费 `spark-media`）                       |

### scripting

| Crate                   | 职责                         |
|-------------------------|------------------------------|
| `spark-gc`              | 脚本堆与标记–清扫 GC         |
| `spark-vm`              | 栈式字节码虚拟机             |
| `spark-jit`             | 热路径字节码特化（基线 JIT） |
| `spark-script`          | 脚本引擎门面（多前端 → VM）  |
| `spark-script-valkyrie` | Valkyrie 前端（Oaks）        |
| `spark-script-lua`      | Lua 前端（Oaks）             |
| `spark-script-ruby`     | Ruby / RGSS 子集前端         |

### gameplay

| Crate                     | 职责                                                                                      |
|---------------------------|-------------------------------------------------------------------------------------------|
| `spark-engine`            | 帧主循环编排 + VM 之上的模组加载、钩子、数据表、本地化服务与资源挂载（模组跑 `spark-vm`） |
| `spark-engine-rts`        | RTS 特异化：选取、指令队列、迷雾骨架                                                      |
| `spark-engine-stg`        | STG 特异化：弹幕池、发射器、判定与关卡时钟                                                |
| `spark-engine-rpg`        | RPG 特异化：队伍、背包、属性、任务、回合序                                                |
| `spark-engine-platformer` | 平台跳跃：刚体、固体/单向台、土狼跳、相机                                                 |
| `spark-engine-galgame`    | Galgame/AVG：对白、选项、旗标、图层槽；默认整合 Live2D 脚本插件                           |
| `spark-plugin`            | 脚本插件机制：向 `spark-vm` 注册原生能力（Rust 侧请直接依赖 crate）                       |
| `spark-plugin-live2d`     | Live2D 脚本插件：模型句柄、参数与动作（可换 Cubism 后端）                                 |
| `spark-plugin-steam`      | Steam 脚本插件：成就、统计、云文件与用户信息（可换 Steamworks 后端）                      |

### bindings

| Crate        | 职责                                                                        |
|--------------|-----------------------------------------------------------------------------|
| `spark-wasm` | Wasm cdylib（C ABI）；产物拷至 `packages/spark-unknown-wasm32`              |
| `spark-napi` | Node-API 原生绑定（feature `node`）；产物发布到 `packages/spark-<platform>` |

### npm 包（`packages/`）

| 包名                   | 职责                                          |
|------------------------|-----------------------------------------------|
| `spark-engine`         | TS 入口：`resolveNativePath` / `loadSpark`    |
| `spark-win32-x64` 等   | 原生二进制袋（`os`/`cpu` + `main` = `.node`） |
| `spark-unknown-wasm32` | Wasm 包（承接 `spark-wasm`）                  |
