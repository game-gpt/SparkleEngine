//! 自 `src/localization.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_engine::*;

use spark_localization::{LocaleId, MessageArgs, MessageRef};
use std::sync::Arc;

use spark_event::EventBus;
use spark_localization::{LocaleChanged, LocaleSnapshot};
fn loc(tag: &str) -> LocaleId {
    LocaleId::parse(tag).unwrap()
}

#[test]
fn commits_only_at_frame_boundary() {
    let available = [loc("en"), loc("zh-Hans")];
    let en = LocaleSnapshot::empty(loc("en"), &loc("en"), &available, 1);
    let mut service = LocalizationService::new(en);
    assert_eq!(service.locale().as_str(), "en");

    let zh = LocaleSnapshot::from_entries(
        loc("zh-Hans"),
        &loc("en"),
        &available,
        0,
        [(Arc::from("ui"), Arc::from("ok"), Arc::from("zh-Hans"), Arc::from("好"))],
    );
    service.queue_snapshot(zh);
    assert!(service.has_pending());
    assert_eq!(service.locale().as_str(), "en");

    let mut bus = EventBus::new();
    let changed = service.commit_pending(&mut bus).unwrap();
    assert_eq!(changed.previous.as_str(), "en");
    assert_eq!(changed.current.as_str(), "zh-Hans");
    assert_eq!(service.locale().as_str(), "zh-Hans");
    assert_eq!(service.format(&MessageRef::named("ui", "ok"), &MessageArgs::new()).text.as_ref(), "好");

    bus.update_all();
    let events: Vec<_> = bus.events::<LocaleChanged>().unwrap().iter().cloned().collect();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].current.as_str(), "zh-Hans");
}

#[test]
fn failed_prepare_keeps_old_snapshot() {
    let available = [loc("en")];
    let en = LocaleSnapshot::from_entries(
        loc("en"),
        &loc("en"),
        &available,
        1,
        [(Arc::from("ui"), Arc::from("ok"), Arc::from("en"), Arc::from("OK"))],
    );
    let mut service = LocalizationService::new(en);
    // 模拟装载失败：不 queue，仅 cancel。
    service.cancel_pending();
    assert_eq!(service.format(&MessageRef::named("ui", "ok"), &MessageArgs::new()).text.as_ref(), "OK");
}
