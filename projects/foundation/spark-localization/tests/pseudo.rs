//! 自 `src/pseudo.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_localization::{LocaleId, LocalizationDocument, MessageDefinition, PseudoKind, generate_pseudo};
use std::sync::Arc;

#[test]
fn accents_expand_ascii_vowels() {
    let mut doc = LocalizationDocument::new(LocaleId::parse("en").unwrap(), "game");
    doc.insert("menu.continue", MessageDefinition::Text(Arc::from("Continue")));
    let pseudo = generate_pseudo(&doc, PseudoKind::Accents);
    let def = pseudo.messages.values().next().unwrap();
    match def {
        MessageDefinition::Text(text) => assert_eq!(text.as_ref(), "Çóñtíñúé"),
        other => panic!("unexpected {other:?}"),
    }
    assert_eq!(pseudo.locale.as_str(), "en-XA");
}
