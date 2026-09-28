# spark-prefab

声明式 Prefab： **serde 领域模型**，磁盘载体为 **VON**（`oak-von`）。本地节点 ID、字段路径覆盖、嵌套引用。Agent 写路径与节点名；GUID
由 `spark-asset` 旁车（同为 VON）维护。

```rust
use spark_prefab::{PrefabDocument, PrefabInstance};
use spark_asset::MetaValue;
use std::collections::BTreeMap;

let mut prefab = PrefabDocument::new("player");
prefab.ensure_child("player", "sprite").unwrap();
prefab
    .set_component(
        "sprite",
        "Sprite",
        MetaValue::Table(BTreeMap::from([(
            "texture".into(),
            MetaValue::String("assets/player.png".into()),
        )])),
    )
    .unwrap();
prefab.validate().unwrap();

let mut inst = PrefabInstance::new("assets/player.von", "player_spawn");
inst.set_override(
    "player/Transform.position",
    MetaValue::Array(vec![MetaValue::Int(100), MetaValue::Int(64)]),
);
```

`save_registered` 写盘并确保旁车 `.meta`（VON，`kind = prefab`，GUID 不因重存而更换）。
`with_overrides` 在副本上应用实例补丁，不回写源 Prefab。

```bash
cargo test -p spark-prefab
```
