//! 自 `src/asset.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_localization::{
    LocaleEntry, LocaleId, LocaleRequest, LocalizationManifest, MemoryLocaleLoader, MessageArgs, MessageRef, prepare_snapshot,
};
use std::sync::Arc;

#[test]
fn loads_json_shards_into_snapshot() {
    let mut loader = MemoryLocaleLoader::new();
    loader.insert(
        "locales/en/common.json",
        r#"{
          "locale": "en",
          "namespace": "game",
          "messages": { "menu.quit": "Quit" }
        }"#,
    );
    loader.insert(
        "locales/zh-Hans/common.json",
        r#"{
          "locale": "zh-Hans",
          "namespace": "game",
          "messages": { "menu.quit": "退出" }
        }"#,
    );

    let mut manifest = LocalizationManifest::new(LocaleId::parse("en").unwrap());
    manifest.locales.push(LocaleEntry { locale: LocaleId::parse("en").unwrap(), fallback: vec![], bundled: true, font_hint: None });
    manifest.locales.push(LocaleEntry { locale: LocaleId::parse("zh-Hans").unwrap(), fallback: vec![], bundled: true, font_hint: None });
    manifest.shards.insert(LocaleId::parse("en").unwrap(), vec![Arc::from("locales/en/common.json")]);
    manifest.shards.insert(LocaleId::parse("zh-Hans").unwrap(), vec![Arc::from("locales/zh-Hans/common.json")]);

    let request = LocaleRequest::new(vec![LocaleId::parse("zh-Hans-CN").unwrap()], vec![]);
    let snap = prepare_snapshot(&loader, &manifest, &request, 7).unwrap();
    assert_eq!(snap.locale.as_str(), "zh-Hans");
    assert_eq!(snap.generation, 7);
    let text = snap.format(&MessageRef::named("game", "menu.quit"), &MessageArgs::new());
    assert_eq!(text.text.as_ref(), "退出");
}
