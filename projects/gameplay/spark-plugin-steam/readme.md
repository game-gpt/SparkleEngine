# spark-plugin-steam

Steam 脚本插件：成就、统计、云文件、用户信息；可挂空后端。

```rust
use spark_plugin_steam::SteamPlugin;
use std::rc::Rc;

let plugin = SteamPlugin::with_null_backend_app(480, "tester");
let rt = Rc::clone(plugin.runtime());
rt.borrow_mut().backend.unlock_achievement("ACH_FIRST").unwrap();
rt.borrow_mut().backend.set_stat("kills", 3.0).unwrap();
rt.borrow_mut().backend.cloud_write("save.txt", "hello").unwrap();
```

注册进 `PluginRegistry` 后供 VM 调用。空后端 `is_available()` 为 false，仍可测逻辑。

```bash
cargo test -p spark-plugin-steam
```
