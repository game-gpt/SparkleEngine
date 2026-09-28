# spark-asset

Asset keys, in-memory cache, and loader interfaces. `BytesLoader` reads bytes from a root directory; decoding is done by other loaders / upper layers.

Sidecar `.meta` ([`AssetMetaStore`]) is **VON** text (`oak-von` serde), storing asset GUID (UUID v7) and optional import settings. Path references are used by agents / scripts; GUIDs are tool-generated only. `load` returns `spark.asset.meta_missing` when the sidecar is absent; it will not silently mint a new identity.

[`AssetRef`] uses paths as the source format; tools `resolve` can attach a `guid`.
[`AssetIndex`] scans sidecars for GUID ↔ path mapping; `rename` moves files and `.meta` while preserving GUID.

```rust
use spark_asset::{AssetCache, BytesLoader};

let loader = BytesLoader::new(&root_dir);
let mut cache = AssetCache::new();
let id = cache.load("a.txt", &loader).unwrap();
let bytes = cache.bytes(id).unwrap();
assert_eq!(cache.generation(id), Some(1));

cache.notify_changed("a.txt", &loader).unwrap();
let _ = cache.drain_reloads();
```

```rust
use spark_asset::AssetMetaStore;

// Register a new asset (fails if sidecar already exists)
let meta = AssetMetaStore::create("assets/image.png")?;
// Load; missing sidecar is an error, no auto-create
let same = AssetMetaStore::load("assets/image.png")?;
assert_eq!(meta.guid, same.guid);
```

Errors: `AssetError` (includes `LoadError`, `AssetMetaError`). Hot-reload polling hooks: `HotReloadWatch` / `ReloadEvent`. Custom formats implement the `AssetLoader` trait.

```bash
cargo test -p spark-asset
```
