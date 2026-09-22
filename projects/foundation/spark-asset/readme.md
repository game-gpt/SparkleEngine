# spark-asset

资源键、内存缓存与加载器接口。`BytesLoader` 从根目录读字节；具体解码由其它 loader / 上层完成。

旁车 `.meta`（[`AssetMetaStore`]）为 **VON** 文本（`oak-von` serde），保存资源 GUID（UUID v7）与可选导入设置。路径引用由 Agent / 脚本使用；GUID 仅由工具生成。`load` 在缺失旁车时返回 `spark.asset.meta_missing`，不会静默换发新身份。

[`AssetRef`] 以路径为源格式；工具 `resolve` 后可附着 `guid`。
[`AssetIndex`] 扫描旁车建立 GUID ↔ 路径映射，`rename` 移动文件与 `.meta` 且保持 GUID。

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

// 新资源登记（旁车已存在则失败）
let meta = AssetMetaStore::create("assets/image.png")?;
// 读取；缺失则报错，不自动 create
let same = AssetMetaStore::load("assets/image.png")?;
assert_eq!(meta.guid, same.guid);
```

错误：`AssetError`（含 `LoadError`、`AssetMetaError`）。热重载轮询钩子见 `HotReloadWatch` / `ReloadEvent`。自定义格式实现
`AssetLoader` trait。

```bash
cargo test -p spark-asset
```
