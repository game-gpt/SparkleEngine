# spark-script-lua

Oaks Lua 子集 → `spark-ir` → 字节码。不是完整 Lua 运行时。

```rust
use spark_script_lua::compile;
use spark_vm::{StdHost, Vm};

let module = compile("return 40 + 2").unwrap();
let v = Vm::new(module).run(&mut StdHost).unwrap();
assert_eq!(v.as_number(), Some(42.0));
```

也有 `compile_with_binds`、`parse`。错误：`LuaScriptError`。table / 元表 / 协程不在当前子集。

```bash
cargo test -p spark-script-lua
```
