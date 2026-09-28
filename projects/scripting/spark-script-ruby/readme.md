# spark-script-ruby

Oaks Ruby 子集 → `spark-ir` → 字节码。

```rust
use spark_script_ruby::compile;
use spark_vm::{StdHost, Vm};

let module = compile("return 40 + 2").unwrap();
let value = Vm::new(module).run(&mut StdHost).unwrap();
assert_eq!(value.as_number(), Some(42.0));
```

错误：`RubyScriptError`。不是完整 MRI / RGSS。

```bash
cargo test -p spark-script-ruby
```
