# spark-plugin-live2d

Live2D 脚本插件：加载模型、读写参数；可换后端。

```rust
use spark_plugin_live2d::Live2dPlugin;
use std::rc::Rc;

let plugin = Live2dPlugin::with_null_backend();
let rt = Rc::clone(plugin.runtime());
let id = rt.borrow_mut().backend.load("model.model3.json").unwrap();
rt.borrow_mut().backend.set_param(id, "ParamAngleX", 0.5).unwrap();
```

经 `PluginRegistry::register` 装进 VM 后，宿主名形如 `plugin.live2d_load`。常量 `LIVE2D_NATIVES` 列出原生名。

```bash
cargo test -p spark-plugin-live2d
```
