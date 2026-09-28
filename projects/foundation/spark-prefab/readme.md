# spark-prefab

Declarative prefabs: **serde domain model**, on-disk carrier is **VON** (`oak-von`). Local node IDs, field-path overrides, nested references. Agents write paths and node names; GUIDs are maintained by `spark-asset` sidecars (also VON).

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

`save_registered` writes to disk and ensures sidecar `.meta` (VON, `kind = prefab`; GUID does not change on re-save).
`with_overrides` applies instance patches on a copy without writing back to the source prefab.

```bash
cargo test -p spark-prefab
```
