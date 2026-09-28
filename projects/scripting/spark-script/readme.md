# spark-script

Multi-language script compile and run facade.

```text
source + HostSchema
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

Errors: `ScriptError` plus link / verify / artifact IO, etc. Production paths do not hand-write `Op` bypassing IR.

```bash
cargo test -p spark-script
```
