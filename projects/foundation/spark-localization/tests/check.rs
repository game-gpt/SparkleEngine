//! 自 `src/check.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_localization::{
    CheckIssueKind, LocaleId, LocalizationDocument, MessageDefinition, MessageNode, SelectKind, check_document, check_locale_set,
};
use std::{collections::BTreeMap, sync::Arc};

#[test]
fn detects_missing_other_case() {
    let mut doc = LocalizationDocument::new(LocaleId::parse("en").unwrap(), "game");
    let mut cases = BTreeMap::new();
    cases.insert(Arc::from("one"), vec![MessageNode::Text(Arc::from("one item"))]);
    doc.insert("item_count", MessageDefinition::Select { argument: Arc::from("count"), kind: SelectKind::Cardinal, cases });
    let report = check_document(&doc);
    assert!(report.issues.iter().any(|i| i.kind == CheckIssueKind::MissingSelectCase));
}

#[test]
fn detects_missing_key_across_locales() {
    let mut en = LocalizationDocument::new(LocaleId::parse("en").unwrap(), "game");
    en.insert("menu.quit", MessageDefinition::Text(Arc::from("Quit")));
    let zh = LocalizationDocument::new(LocaleId::parse("zh-Hans-CN").unwrap(), "game");
    let report = check_locale_set(&en, &[zh]);
    assert!(report.issues.iter().any(|i| i.kind == CheckIssueKind::MissingKey));
}

#[test]
fn issues_carry_tokens_not_prose() {
    let mut en = LocalizationDocument::new(LocaleId::parse("en").unwrap(), "game");
    en.insert("a", MessageDefinition::Text(Arc::from("A")));
    let mut zh = LocalizationDocument::new(LocaleId::parse("zh-Hans-CN").unwrap(), "other");
    zh.insert("a", MessageDefinition::Text(Arc::from("甲")));
    let report = check_locale_set(&en, &[zh]);
    let issue = report.issues.iter().find(|i| i.kind == CheckIssueKind::NamespaceMismatch).expect("namespace mismatch");
    let token = issue.token.as_deref().unwrap_or("");
    assert!(token.contains("!="));
    assert!(!token.contains(' '));
}
