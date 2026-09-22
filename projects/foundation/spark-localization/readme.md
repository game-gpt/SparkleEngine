# spark-localization

Locale、消息文档、编译与不可变快照。

```rust
use spark_localization::{
    LocaleId, LocaleRequest, LocalizationManifest, MemoryLocaleLoader, MessageArgs, MessageRef,
    prepare_snapshot,
};
use std::sync::Arc;

let mut loader = MemoryLocaleLoader::new();
loader.insert(
    "locales/en/common.json",
    r#"{"locale":"en","namespace":"game","messages":{"menu.quit":"Quit"}}"#,
);

let mut manifest = LocalizationManifest::new(LocaleId::parse("en").unwrap());
// 填 locales / shards 后：
let request = LocaleRequest::new(vec![LocaleId::parse("en").unwrap()], vec![]);
let snap = prepare_snapshot(&loader, &manifest, &request, 1).unwrap();
let _ = snap.format(&MessageRef::named("game", "menu.quit"), &MessageArgs::new());
```

另有 `compile_document`、`negotiate`、`Localizer`、伪本地化与覆盖率检查。错误类型按阶段拆分（`CompileError`、`LocaleLoadError`
等）。

```bash
cargo test -p spark-localization
```
