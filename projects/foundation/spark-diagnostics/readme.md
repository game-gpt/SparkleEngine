# spark-diagnostics

Structured errors and diagnostics: `Error`, `ErrorCode`, typed parameters `ErrorArg` / `ErrorArgs`, plus `Diagnostic`, `SourceSpan`, `MessageKey`.

```rust
use spark_diagnostics::{Error, ErrorArg, ErrorArgs, ErrorCode};

let code = ErrorCode::parse("spark.example.demo").unwrap();
let err = Error::new(code).arg("path", ErrorArg::String("a.txt".into()));
assert_eq!(err.to_string(), "spark.example.demo");

let _ = ErrorArgs::new().with("n", ErrorArg::Unsigned(3));
```

Preset codes live in the `codes` module. Upper crates (including `spark-types::SparkError`) build on this model; human-readable text is produced by localization / UI from codes and arguments.

```bash
cargo test -p spark-diagnostics
```
