# spark-script

多语言脚本编译与运行门面。

```text
源码 + HostSchema
  → spark-script-valkyrie / lua / ruby
  → spark-ir
  → SparkObject / ExecutableImage
  → ScriptRuntime → spark-vm
```

```rust
use spark_script::{HostSchema, ScriptCompiler, ScriptLanguage};

let host = HostSchema::new(1);
let mut compiler = ScriptCompiler::new();
let compiled = compiler
    .compile_source(ScriptLanguage::Valkyrie, "return 1 + 2", &host)
    .unwrap();

use spark_script::{CompilationRequest, ScriptRuntime, SparkObject};
let req = CompilationRequest::repl(ScriptLanguage::Valkyrie, "return 1 + 2", host.clone());
let obj = compiler.compile_object(&req).unwrap();
let package = compiler.link_objects(&[obj], &host, &req.package).unwrap();
let mut rt = ScriptRuntime::from_image(&package.image, &host).unwrap();
assert_eq!(rt.call_on_load_std().unwrap().as_number(), Some(3.0));
```

错误：`ScriptError` 及链接 / 校验 / 制品 IO 等。正式路径不绕过 IR 手写 `Op`。

```bash
cargo test -p spark-script
```
