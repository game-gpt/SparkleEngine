# spark-script-lua

Oaks Lua subset → `spark-ir` → bytecode. Not a full Lua runtime.

```rust
use spark_script_lua::compile;
use spark_vm::{StdHost, Vm};

let module = compile("return 40 + 2").unwrap();
let v = Vm::new(module).run(&mut StdHost).unwrap();
assert_eq!(v.as_number(), Some(42.0));
```

Also `compile_with_binds`, `parse`. Errors: `LuaScriptError`. Tables / metatables / coroutines are outside the current subset.

```bash
cargo test -p spark-script-lua
```
