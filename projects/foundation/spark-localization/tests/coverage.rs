//! 自 `src/coverage.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_localization::{LocaleId, LocalizationDocument, MessageDefinition, coverage_against};
use std::sync::Arc;

#[test]
fn reports_missing_and_present() {
    let mut en = LocalizationDocument::new(LocaleId::parse("en").unwrap(), "game");
    en.insert("a", MessageDefinition::Text(Arc::from("A")));
    en.insert("b", MessageDefinition::Text(Arc::from("B")));
    let mut zh = LocalizationDocument::new(LocaleId::parse("zh-Hans").unwrap(), "game");
    zh.insert("a", MessageDefinition::Text(Arc::from("甲")));
    let report = coverage_against(&en, &zh);
    assert_eq!(report.present, 1);
    assert_eq!(report.missing, 1);
    assert!(report.ratio_present() < 1.0);
}
