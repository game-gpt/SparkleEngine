//! 自 `src/document_von.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_localization::{MessageDefinition, MessageName, document_from_von_str};

#[test]
fn parses_flat_messages() {
    let doc = document_from_von_str(
        r#"
locale = "zh-Hans"
namespace = "demo"
message.menu.quit = "退出"
message.hello = "你好，{name}"
"#,
    )
    .unwrap();
    assert_eq!(doc.locale.as_str(), "zh-Hans");
    assert_eq!(doc.namespace.as_str(), "demo");
    assert!(matches!(
        doc.messages.get(&MessageName::new("menu.quit")),
        Some(MessageDefinition::Text(t)) if t.as_ref() == "退出"
    ));
    assert!(matches!(doc.messages.get(&MessageName::new("hello")), Some(MessageDefinition::Pattern(_))));
}
