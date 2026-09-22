//! 自 `src/compile.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_localization::{CompiledMessage, LocaleId, LocalizationDocument, MessageDefinition, compile_document};
use std::sync::Arc;

#[test]
fn compiles_text_and_template_sugar() {
    let mut doc = LocalizationDocument::new(LocaleId::parse("en").unwrap(), "game");
    doc.insert("plain", MessageDefinition::Text(Arc::from("Hi")));
    doc.insert("hello", MessageDefinition::Text(Arc::from("Hello, {name}")));
    let bundle = compile_document(&doc).unwrap();
    assert_eq!(bundle.len(), 2);
    assert!(matches!(bundle.get_named("game", "plain", &doc.locale), Some(CompiledMessage::Text(_))));
    assert!(matches!(bundle.get_named("game", "hello", &doc.locale), Some(CompiledMessage::Pattern(_))));
    assert_ne!(bundle.content_hash, 0);
}
