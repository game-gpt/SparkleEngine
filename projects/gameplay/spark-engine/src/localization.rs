//! 引擎侧本地化服务：准备快照、帧边界原子提交、经事件总线广播。

use spark_event::EventBus;
use spark_localization::{LocaleChanged, LocaleId, LocaleSnapshot, LocalizedText, Localizer, MessageArgs, MessageRef};

/// 持有当前 [`Localizer`]，并把待切换快照推迟到帧边界提交。
#[derive(Debug)]
pub struct LocalizationService {
    localizer: Localizer,
    pending: Option<LocaleSnapshot>,
    next_generation: u64,
}

impl Default for LocalizationService {
    fn default() -> Self {
        Self::empty(LocaleId::parse("en").expect("default locale"))
    }
}

impl LocalizationService {
    pub fn new(initial: LocaleSnapshot) -> Self {
        let next_generation = initial.generation.saturating_add(1);
        Self { localizer: Localizer::new(initial), pending: None, next_generation }
    }

    /// 以产品默认 Locale 的空快照启动。
    pub fn empty(product_default: LocaleId) -> Self {
        let available = [product_default.clone()];
        let snap = LocaleSnapshot::empty(product_default.clone(), &product_default, &available, 1);
        Self::new(snap)
    }

    pub fn localizer(&self) -> &Localizer {
        &self.localizer
    }

    pub fn snapshot(&self) -> std::sync::Arc<LocaleSnapshot> {
        self.localizer.snapshot()
    }

    pub fn generation(&self) -> u64 {
        self.localizer.generation()
    }

    pub fn locale(&self) -> &LocaleId {
        self.localizer.locale()
    }

    pub fn format(&self, message: &MessageRef, args: &MessageArgs) -> LocalizedText {
        self.localizer.format(message, args)
    }

    /// 排队下一代快照；不立即替换当前只读视图。
    ///
    /// 若 `snapshot.generation == 0`，自动分配单调递增 generation。
    pub fn queue_snapshot(&mut self, mut snapshot: LocaleSnapshot) {
        if snapshot.generation == 0 {
            snapshot.generation = self.next_generation;
        }
        self.next_generation = snapshot.generation.saturating_add(1);
        self.pending = Some(snapshot);
    }

    pub fn has_pending(&self) -> bool {
        self.pending.is_some()
    }

    /// 丢弃未提交的待切换快照（例如装载失败后的清理）。
    pub fn cancel_pending(&mut self) {
        self.pending = None;
    }

    /// 在帧边界提交待切换快照，并向总线写入 [`LocaleChanged`]。
    ///
    /// 无 pending 时返回 `None` 且不碰总线。
    pub fn commit_pending(&mut self, bus: &mut EventBus) -> Option<LocaleChanged> {
        let next = self.pending.take()?;
        let changed = self.localizer.commit(next);
        bus.send(changed.clone());
        Some(changed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use spark_localization::{LocaleId, MessageArgs, MessageRef};
    use std::sync::Arc;

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
}
