# spark-plugin-steam

Steam script plugin: achievements, stats, cloud files, user info; optional null backend.

```rust
use spark_plugin_steam::SteamPlugin;
use std::rc::Rc;

let plugin = SteamPlugin::with_null_backend_app(480, "tester");
let rt = Rc::clone(plugin.runtime());
rt.borrow_mut().backend.unlock_achievement("ACH_FIRST").unwrap();
rt.borrow_mut().backend.set_stat("kills", 3.0).unwrap();
rt.borrow_mut().backend.cloud_write("save.txt", "hello").unwrap();
```

After registration in `PluginRegistry`, available to the VM. Null backend `is_available()` is false; logic can still be tested.

```bash
cargo test -p spark-plugin-steam
```
