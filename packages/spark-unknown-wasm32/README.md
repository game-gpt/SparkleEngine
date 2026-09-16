# `spark-unknown-wasm32`

Spark Engine 的 **Wasm** 平台 npm 包（Rust target：`wasm32-unknown-unknown`）。

Wasm 二进制由 Cargo 包 **`spark-wasm`** 产出，再拷贝为本包的 `spark_engine_bg.wasm`：

```bash
rustup target add wasm32-unknown-unknown
cargo build -p spark-wasm --target wasm32-unknown-unknown --release
npm run copy:wasm -w spark-unknown-wasm32
# 或：npm run build:wasm -w spark-unknown-wasm32
```

- 浏览器 / 打包器：`import { loadSpark } from "spark-unknown-wasm32"`
- 导出 ABI：`spark_vec2_length`、`spark_version_code`（见 `spark-wasm`）
- 未放入 `.wasm` 时 `loadSpark` 提供纯 JS 几何回退，便于 TS 联调
