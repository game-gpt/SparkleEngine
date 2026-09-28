# spark-ir

语言无关中间表示：HIR → MIR，再 `emit_module` 生成 `spark-vm` 字节码。

```rust
use spark_ir::{emit_module, lower_module};
// 从前端得到 HirModule 后：
// let mir = lower_module(&hir)?;
// let module = emit_module(&mir)?;
// Vm::new(module).run(...)
```

宿主绑定：`HostBindTable`、`HostId`、`emit_module_with_host`。测例 `tests/codegen.rs` 覆盖算术管线。语言前端在
`spark-script-*`；本 crate 不含源语言枚举。

```bash
cargo test -p spark-ir
```
