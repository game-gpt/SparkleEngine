# spark-prefab

声明式 Prefab：本地节点 ID、字段路径覆盖、嵌套引用。Agent 写路径与节点名；GUID 由 `spark-asset` 旁车维护。

```rust
use spark_prefab::{PrefabDocument, PrefabInstance};
use serde_json::json;

let mut prefab = PrefabDocument::new("player");
prefab.ensure_child("player", "sprite").unwrap();
prefab
    .set_component("sprite", "Sprite", json!({ "texture": "assets/player.png" }))
    .unwrap();
prefab.validate().unwrap();

let mut inst = PrefabInstance::new("assets/player.prefab", "player_spawn");
inst.set_override("player/Transform.position", json!([100, 64]));
```

```bash
cargo test -p spark-prefab
```
