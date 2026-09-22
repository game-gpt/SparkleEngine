//! 自 `src/manifest_von.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_localization::*;

#[test]
fn parses_localization_von() {
    let text = r#"
# 产品本地化清单
product_default = "en"
cldr_version = "45"
format_version = 1
locales = ["en", "zh-Hans", "ar"]
bundled = ["en", "zh-Hans"]
namespace = ["astracraft", "AstraCraft", false]
"#;
    let m = manifest_from_von_str(text).unwrap();
    assert_eq!(m.product_default.as_str(), "en");
    assert_eq!(m.locales.len(), 3);
    assert!(m.locales.iter().any(|e| e.locale.as_str() == "ar" && !e.bundled));
    assert_eq!(m.namespaces[0].namespace.as_str(), "astracraft");
    assert!(!m.namespaces[0].allow_override);
    assert!(m.shards.is_empty());
}

#[test]
fn parses_shard_paths() {
    let text = r#"
product_default = "en"
shards = ["locales/en.von", "locales/zh-Hans.von"]
"#;
    let m = manifest_from_von_str(text).unwrap();
    let paths = m.shards.get(&m.product_default).unwrap();
    assert_eq!(paths.len(), 2);
    assert_eq!(paths[0].as_ref(), "locales/en.von");
}
