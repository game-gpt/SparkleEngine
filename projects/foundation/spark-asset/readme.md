# spark-asset

资源键、内存缓存与加载器接口。`BytesLoader` 从根目录读字节；具体解码由其它 loader / 上层完成。

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

错误：`AssetError`（含 `LoadError::NotFound` 等）。热重载轮询钩子见 `HotReloadWatch` / `ReloadEvent`。自定义格式实现
`AssetLoader` trait。

```bash
cargo test -p spark-asset
```
