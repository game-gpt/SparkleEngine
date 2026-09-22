//! 自 `src/locale.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_localization::*;

fn loc(tag: &str) -> LocaleId {
    LocaleId::parse(tag).unwrap()
}

#[test]
fn parse_normalizes_bcp47() {
    let id = loc("zh-hans-cn");
    assert_eq!(id.as_str(), "zh-Hans-CN");
    assert_eq!(id.language(), "zh");
    assert_eq!(id.script(), Some("Hans"));
    assert_eq!(id.region(), Some("CN"));
}

#[test]
fn parse_strips_unicode_extension() {
    let id = loc("en-US-u-ca-buddhist");
    assert_eq!(id.as_str(), "en-US");
}

#[test]
fn negotiate_is_deterministic() {
    let available = vec![loc("en"), loc("zh-Hans"), loc("ar")];
    let request = LocaleRequest::new(vec![loc("zh-Hans-CN")], vec![]);
    let a = negotiate(&request, &available, &loc("en"));
    let b = negotiate(&request, &available, &loc("en"));
    assert_eq!(a, b);
    assert_eq!(a.as_str(), "zh-Hans");
}

#[test]
fn negotiate_falls_back_to_language() {
    let available = vec![loc("ru"), loc("en")];
    let request = LocaleRequest::new(vec![loc("ru-RU")], vec![]);
    assert_eq!(negotiate(&request, &available, &loc("en")).as_str(), "ru");
}

#[test]
fn rtl_guess_for_arabic() {
    assert_eq!(loc("ar").direction(), TextDirection::Rtl);
    assert_eq!(loc("zh-Hans-CN").direction(), TextDirection::Ltr);
}
