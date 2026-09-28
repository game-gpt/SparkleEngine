# spark-plugin-live2d

Live2D script plugin: load models, read/write parameters; swappable backend.

```rust
use spark_plugin_live2d::Live2dPlugin;
use std::rc::Rc;

let plugin = Live2dPlugin::with_null_backend();
let rt = Rc::clone(plugin.runtime());
let id = rt.borrow_mut().backend.load("model.model3.json").unwrap();
rt.borrow_mut().backend.set_param(id, "ParamAngleX", 0.5).unwrap();
```

After `PluginRegistry::register`, host names look like `plugin.live2d_load`. Constant `LIVE2D_NATIVES` lists native names.

```bash
cargo test -p spark-plugin-live2d
```
