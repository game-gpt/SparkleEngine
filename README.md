# Spark Engine

Spark is a Rust game engine library stack: ECS, fixed-step timing, input, 2D/3D rendering contracts, retained widgets, script compilation and VM, plus Node.js / Wasm host bindings. The engine layer carries no gameplay data.

The product npm package is `@game-gpt/sparkle-engine`; the CLI entry is `spark`. Window events and GPU submission live in `spark-renderer-wgpu`; **the authoritative world and frame scheduler live in `SparkRuntime` (`spark-engine`)**.

## Environment and build

- Rust toolchain: root `rust-toolchain.toml`
- Node.js ≥ 18, pnpm

```bash
pnpm install
pnpm run build:ts

cargo run -p ping-pong          # ping-pong sample (SparkRuntime)
cargo run -p snake              # snake sample (SparkRuntime)
cargo run -p spark-studio       # editor
# or pnpm exec spark studio

pnpm run build:napi             # native .node (as needed)
rustup target add wasm32-unknown-unknown
pnpm run build:wasm             # Wasm platform bundle (as needed)
```

## Module map

| Task | crate |
|------|-------|
| Entities / components / scheduling | `spark-ecs` |
| Fixed-step clock | `spark-time` |
| Keyboard and mouse frame state | `spark-input` |
| Draw lists and frame context | `spark-renderer` |
| GPU texture descriptors and upload packets | `spark-texture` |
| PNG / JPEG / WebP / KTX2 / DDS decode | `spark-png` / `spark-jpeg` / `spark-webp` / `spark-ktx2` / `spark-dds` |
| wgpu window and submission | `spark-renderer-wgpu` |
| Retained UI | `spark-widget` |
| Script compile and execute | `spark-script` → `spark-vm` |
| Runtime and mod shell | `spark-engine` (`SparkRuntime` / `run_runtime`) |
| Node bindings | `spark-napi` |
| Wasm ABI | `spark-wasm` |
| glTF import | `spark-gltf` |

Per-crate notes live in `projects/**/readme.md`.

## Runtime path (example)

`ping-pong` `main`:

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

Games register systems on `SparkRuntime` via `NativeGamePlugin`; state lives in `World` resources and components. `run_runtime` drives the frame loop and calls `spark-renderer-wgpu` to open the window and submit.

Custom editor shells use `run_window_2d` + `WindowPump2d` (see `spark-studio`).

## Notes

- `spark-napi` N-API exports require `--features node`; the default feature set is empty for pure Rust tests.
- `spark-jit` currently specializes bytecode; it is not a full machine-code backend.
- Lua / Ruby frontends are language subsets, not full language runtimes.
