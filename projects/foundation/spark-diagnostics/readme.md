# spark-diagnostics

结构化错误与诊断模型：`Error`、`ErrorCode`、类型化参数 `ErrorArg` / `ErrorArgs`，以及 `Diagnostic`、`SourceSpan`、
`MessageKey`。

```rust
use spark_diagnostics::{Error, ErrorArg, ErrorArgs, ErrorCode};

let code = ErrorCode::parse("spark.example.demo").unwrap();
let err = Error::new(code).arg("path", ErrorArg::String("a.txt".into()));
assert_eq!(err.to_string(), "spark.example.demo");

let _ = ErrorArgs::new().with("n", ErrorArg::Unsigned(3));
```

预置码在 `codes` 模块。上层 crate（含 `spark-types::SparkError`）都建立在这套模型上；给人看的文案由 localization / UI
根据码与参数生成。

```bash
cargo test -p spark-diagnostics
```
