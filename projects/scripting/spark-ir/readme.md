# spark-ir

Language-neutral intermediate representation: HIR → MIR, then `emit_module` generates `spark-vm` bytecode.

```rust
use spark_ir::{emit_module, lower_module};
// After obtaining HirModule from a frontend:
// let mir = lower_module(&hir)?;
// let module = emit_module(&mir)?;
// Vm::new(module).run(...)
```

Host bindings: `HostBindTable`, `HostId`, `emit_module_with_host`. Test `tests/codegen.rs` covers the arithmetic pipeline. Language frontends live in `spark-script-*`; this crate has no source-language enum.

```bash
cargo test -p spark-ir
```
