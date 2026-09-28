# spark-script-valkyrie

Oaks Valkyrie → HIR → `spark-ir` → `spark-vm` bytecode.

```rust
use spark_script_valkyrie::compile;
use spark_vm::{StdHost, Vm};

let module = compile("return 40 + 2").unwrap();
let mut vm = Vm::new(module);
let v = vm.run(&mut StdHost).unwrap();
assert_eq!(v.as_number(), Some(42.0));
```

Also `compile_with_binds`, `parse`, `list_micros`. Errors: `ValkyrieScriptError` (Parse / Compile). Native parameter descriptors: `NativeParam` / `TypeRef`.

```bash
cargo test -p spark-script-valkyrie
```
