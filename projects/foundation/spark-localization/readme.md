# spark-localization

Locale, message documents, compilation, and immutable snapshots.

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
// Fill locales / shards, then:
let request = LocaleRequest::new(vec![LocaleId::parse("en").unwrap()], vec![]);
let snap = prepare_snapshot(&loader, &manifest, &request, 1).unwrap();
let _ = snap.format(&MessageRef::named("game", "menu.quit"), &MessageArgs::new());
```

Also `compile_document`, `negotiate`, `Localizer`, pseudo-localization, and coverage checks. Error types are split by stage (`CompileError`, `LocaleLoadError`, etc.).

```bash
cargo test -p spark-localization
```
