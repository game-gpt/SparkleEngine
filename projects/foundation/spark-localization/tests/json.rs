//! 自 `src/json.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_localization::{LocaleSnapshot, MessageArgs, MessageRef, MessageValue, compile_document, document_from_json_str};

#[test]
fn parses_simple_and_rich_json() {
    let json = r#"{
      "locale": "zh-Hans-CN",
      "namespace": "astracraft",
      "messages": {
        "menu.continue": "继续游戏",
        "player.welcome": { "value": "欢迎回来，{player_name}" },
        "inventory.item_count": {
          "select": {
            "argument": "count",
            "kind": "cardinal",
            "cases": {
              "one": [{ "argument": "count", "format": "number" }, " item"],
              "other": [{ "argument": "count", "format": "number" }, " items"]
            }
          }
        }
      }
    }"#;
    let doc = document_from_json_str(json).unwrap();
    assert_eq!(doc.locale.as_str(), "zh-Hans-CN");
    assert_eq!(doc.messages.len(), 3);
    let bundle = compile_document(&doc).unwrap();
    let snap = LocaleSnapshot::from_bundle(doc.locale.clone(), &doc.locale, &[doc.locale.clone()], 1, bundle);
    let mut args = MessageArgs::new();
    args.insert("count", MessageValue::Integer(1));
    let text = snap.format(&MessageRef::named("astracraft", "inventory.item_count"), &args);
    assert_eq!(text.text.as_ref(), "1 item");
}
