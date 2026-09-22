# spark-core

基础类型与错误别名：`Vec2`、`Rect`、`Color`，以及 `SparkError`（即 `spark-diagnostics::Error`）。另含 2D 光照网格
`LightGrid2d` / `LightRgb`。

```rust
use spark_core::{Color, Rect, Vec2};

let p = Vec2::new(3.0, 4.0);
let r = Rect::new(0.0, 0.0, 10.0, 10.0);
assert!(r.contains(p));
let _ = Color::rgb(0.2, 0.3, 0.4);
```

`Rect` 还提供 `intersect` / `intersects` / `center`。诊断侧再导出 `ErrorArg`、`ErrorArgs`、`codes`、`Diagnostic` 等；
`Display` 对错误只打印稳定码。

```bash
cargo test -p spark-core
```
