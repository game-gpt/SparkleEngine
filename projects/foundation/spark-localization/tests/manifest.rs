//! 自 `src/manifest.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_localization::*;
use std::sync::Arc;

#[test]
fn bundled_filter() {
    let mut manifest = LocalizationManifest::new(LocaleId::parse("en").unwrap());
    manifest.locales.push(LocaleEntry { locale: LocaleId::parse("en").unwrap(), fallback: vec![], bundled: true, font_hint: None });
    manifest.locales.push(LocaleEntry {
        locale: LocaleId::parse("ja-JP").unwrap(),
        fallback: vec![],
        bundled: false,
        font_hint: Some(Arc::from("fonts/cjk")),
    });
    assert_eq!(manifest.bundled_locales().len(), 1);
    assert_eq!(manifest.available_locales().len(), 2);
}
