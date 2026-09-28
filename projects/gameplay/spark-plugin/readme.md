# spark-plugin

Plugin table for registering script-side native functions on `spark-vm`.

```rust
use spark_plugin::{Plugin, PluginInfo, PluginRegistry};
use spark_vm::Vm;

struct EchoPlugin;
impl Plugin for EchoPlugin {
    fn info(&self) -> PluginInfo {
        PluginInfo { id: "echo", version: "0.0.0", description: "echo" }
    }
    fn native_names(&self) -> &'static [&'static str] {
        &["echo_ping"]
    }
    fn install(&mut self, vm: &mut Vm) {
        vm.register_native("echo_ping", |_ctx, args| {
            Ok(args.into_iter().next().unwrap_or(spark_gc::Value::Null))
        });
    }
}

let mut reg = PluginRegistry::new();
reg.register(Box::new(EchoPlugin)).unwrap();
// reg.install_all(&mut vm);
```

Errors: `PluginError`. Platform-specific plugins: `spark-plugin-live2d` / `spark-plugin-steam`.

```bash
cargo test -p spark-plugin
```
