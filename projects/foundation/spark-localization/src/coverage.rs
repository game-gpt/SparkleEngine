//! 翻译覆盖率：按 Locale / 命名空间汇总缺失、多余与仅 fallback 命中。

use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

use crate::{
    document::{LocalizationDocument, MessageName},
    locale::LocaleId,
    message::NamespaceId,
};

/// 单条消息相对基线的状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CoverageStatus {
    /// 当前 Locale 有独立译文。
    Present,
    /// 基线有、当前 Locale 缺失。
    Missing,
    /// 当前 Locale 有、基线没有。
    Extra,
    /// 仅能通过 fallback Locale 命中（调用方标记）。
    FallbackOnly,
}

/// 单条覆盖率行。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoverageEntry {
    pub namespace: NamespaceId,
    pub locale: LocaleId,
    pub message: MessageName,
    pub status: CoverageStatus,
}

/// 覆盖率汇总。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CoverageReport {
    pub entries: Vec<CoverageEntry>,
    pub present: usize,
    pub missing: usize,
    pub extra: usize,
    pub fallback_only: usize,
}

impl CoverageReport {
    pub fn ratio_present(&self) -> f32 {
        let denom = (self.present + self.missing + self.fallback_only).max(1) as f32;
        self.present as f32 / denom
    }
}

/// 以 `baseline` 为权威键集，计算 `target` 覆盖率。
pub fn coverage_against(baseline: &LocalizationDocument, target: &LocalizationDocument) -> CoverageReport {
    let mut report = CoverageReport::default();
    let base_keys: BTreeSet<_> = baseline.messages.keys().cloned().collect();
    let target_keys: BTreeSet<_> = target.messages.keys().cloned().collect();

    for key in &base_keys {
        let status = if target_keys.contains(key) { CoverageStatus::Present } else { CoverageStatus::Missing };
        push_entry(&mut report, target, key.clone(), status);
    }
    for key in target_keys.difference(&base_keys) {
        push_entry(&mut report, target, key.clone(), CoverageStatus::Extra);
    }
    report
}

/// 合并多 Locale 报告，并附加已知 fallback-only 键。
pub fn coverage_set(
    baseline: &LocalizationDocument,
    targets: &[LocalizationDocument],
    fallback_only: &BTreeMap<(LocaleId, MessageName), ()>,
) -> CoverageReport {
    let mut merged = CoverageReport::default();
    for target in targets {
        let mut part = coverage_against(baseline, target);
        for entry in &mut part.entries {
            let key = (entry.locale.clone(), entry.message.clone());
            if fallback_only.contains_key(&key) && entry.status == CoverageStatus::Missing {
                entry.status = CoverageStatus::FallbackOnly;
            }
        }
        // 重新计数
        recount(&mut part);
        merged.entries.extend(part.entries);
        merged.present += part.present;
        merged.missing += part.missing;
        merged.extra += part.extra;
        merged.fallback_only += part.fallback_only;
    }
    merged
}

fn push_entry(report: &mut CoverageReport, doc: &LocalizationDocument, message: MessageName, status: CoverageStatus) {
    match status {
        CoverageStatus::Present => report.present += 1,
        CoverageStatus::Missing => report.missing += 1,
        CoverageStatus::Extra => report.extra += 1,
        CoverageStatus::FallbackOnly => report.fallback_only += 1,
    }
    report.entries.push(CoverageEntry { namespace: doc.namespace.clone(), locale: doc.locale.clone(), message, status });
}

fn recount(report: &mut CoverageReport) {
    report.present = 0;
    report.missing = 0;
    report.extra = 0;
    report.fallback_only = 0;
    for entry in &report.entries {
        match entry.status {
            CoverageStatus::Present => report.present += 1,
            CoverageStatus::Missing => report.missing += 1,
            CoverageStatus::Extra => report.extra += 1,
            CoverageStatus::FallbackOnly => report.fallback_only += 1,
        }
    }
}

/// 便于测试构造 fallback-only 标记键。
pub fn fallback_key(locale: LocaleId, message: impl Into<Arc<str>>) -> (LocaleId, MessageName) {
    (locale, MessageName::new(message))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{document::MessageDefinition, locale::LocaleId};

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
}
