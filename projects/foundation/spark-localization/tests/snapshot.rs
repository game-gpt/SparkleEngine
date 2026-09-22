//! 自 `src/snapshot.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_localization::{
    ArgumentFormat, CompileOptions, DiagnosticFlags, LocaleId, LocaleRequest, LocaleSnapshot, LocalizationDocument, Localizer, MessageArgs,
    MessageDefinition, MessageNode, MessageRef, MessageValue, SelectKind, compile_documents, negotiate,
};
use std::{collections::BTreeMap, sync::Arc};

fn loc(tag: &str) -> LocaleId {
    LocaleId::parse(tag).unwrap()
}

#[test]
fn commit_is_atomic_and_bumps_generation() {
    let available = [loc("en"), loc("zh-Hans-CN")];
    let en = LocaleSnapshot::empty(loc("en"), &loc("en"), &available, 1);
    let mut localizer = Localizer::new(en);
    let zh = LocaleSnapshot::from_entries(
        loc("zh-Hans-CN"),
        &loc("en"),
        &available,
        2,
        [(Arc::from("astracraft"), Arc::from("menu.continue"), Arc::from("zh-Hans-CN"), Arc::from("继续游戏"))],
    );
    let changed = localizer.commit(zh);
    assert_eq!(changed.previous.as_str(), "en");
    assert_eq!(changed.current.as_str(), "zh-Hans-CN");
    assert_eq!(changed.generation, 2);
    let text = localizer.format(&MessageRef::named("astracraft", "menu.continue"), &MessageArgs::new());
    assert_eq!(text.text.as_ref(), "继续游戏");
    assert_eq!(text.generation, 2);
}

#[test]
fn format_uses_fallback_chain() {
    let available = [loc("en"), loc("zh-Hans")];
    let resolved = negotiate(&LocaleRequest::new(vec![loc("zh-Hans-CN")], vec![]), &available, &loc("en"));
    assert_eq!(resolved.as_str(), "zh-Hans");
    let snap = LocaleSnapshot::from_entries(
        resolved,
        &loc("en"),
        &available,
        1,
        [(Arc::from("spark"), Arc::from("widget.ok"), Arc::from("en"), Arc::from("OK"))],
    );
    let text = snap.format(&MessageRef::named("spark", "widget.ok"), &MessageArgs::new());
    assert_eq!(text.text.as_ref(), "OK");
    assert!(text.diagnostics.contains(DiagnosticFlags::FALLBACK_USED));
}

#[test]
fn format_substitutes_named_args() {
    let available = [loc("en")];
    let snap = LocaleSnapshot::from_entries(
        loc("en"),
        &loc("en"),
        &available,
        1,
        [(Arc::from("game"), Arc::from("welcome"), Arc::from("en"), Arc::from("Hello, {player_name}"))],
    );
    let mut args = MessageArgs::new();
    args.insert("player_name", MessageValue::String(Arc::from("Ada")));
    let text = snap.format(&MessageRef::named("game", "welcome"), &args);
    assert_eq!(text.text.as_ref(), "Hello, Ada");
}

#[test]
fn format_cardinal_select_from_bundle() {
    let mut cases = BTreeMap::new();
    cases.insert(
        Arc::from("one"),
        vec![MessageNode::Argument { name: Arc::from("count"), format: ArgumentFormat::None }, MessageNode::Text(Arc::from(" item"))],
    );
    cases.insert(
        Arc::from("other"),
        vec![MessageNode::Argument { name: Arc::from("count"), format: ArgumentFormat::None }, MessageNode::Text(Arc::from(" items"))],
    );
    let mut doc = LocalizationDocument::new(loc("en"), "game");
    doc.insert("item_count", MessageDefinition::Select { argument: Arc::from("count"), kind: SelectKind::Cardinal, cases });
    let bundle = compile_documents(&[doc], CompileOptions::default()).unwrap().bundle;
    let snap = LocaleSnapshot::from_bundle(loc("en"), &loc("en"), &[loc("en")], 1, bundle);
    let mut args = MessageArgs::new();
    args.insert("count", MessageValue::Integer(2));
    let text = snap.format(&MessageRef::named("game", "item_count"), &args);
    assert_eq!(text.text.as_ref(), "2 items");
}
