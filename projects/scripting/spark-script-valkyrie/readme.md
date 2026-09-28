# spark-script-valkyrie

Oaks Valkyrie → HIR → `spark-ir` → `spark-vm` 字节码。

```rust
use spark_script_valkyrie::compile;
use spark_vm::{StdHost, Vm};

let module = compile("return 40 + 2").unwrap();
let mut vm = Vm::new(module);
let v = vm.run(&mut StdHost).unwrap();
assert_eq!(v.as_number(), Some(42.0));
```

也有 `compile_with_binds`、`parse`、`list_micros`。错误：`ValkyrieScriptError`（Parse / Compile）。原生参数描述：`NativeParam` /
`TypeRef`。

```bash
cargo test -p spark-script-valkyrie
```
