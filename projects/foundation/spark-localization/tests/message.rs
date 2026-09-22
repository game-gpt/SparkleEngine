//! 自 `src/message.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_localization::*;

#[test]
fn message_ref_display() {
    let r = MessageRef::named("astracraft", "menu.continue");
    assert_eq!(r.to_string(), "astracraft.menu.continue");
    assert!(!r.namespace.is_spark_reserved());
    assert!(NamespaceId::new("spark.widget").is_spark_reserved());
}

#[test]
fn args_are_named() {
    let mut args = MessageArgs::new();
    args.insert("count", MessageValue::Integer(12));
    assert_eq!(args.get("count"), Some(&MessageValue::Integer(12)));
}
