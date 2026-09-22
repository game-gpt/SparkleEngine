//! 自 `src/manifest.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_engine::*;

#[test]
fn parses_flat_mod_von() {
    let raw = r#"
# AstraCraft 原版
id = "demo"
name = "Demo"
version = "0.0.0"
entry = "main.vk"
language = "valkyrie"
dependencies = ["core", "extra"]
"#;
    let m = parse_mod_von(raw).unwrap();
    assert_eq!(m.id, "demo");
    assert_eq!(m.name, "Demo");
    assert_eq!(m.entry.as_deref(), Some("main.vk"));
    assert_eq!(m.dependencies, vec!["core", "extra"]);
}

#[test]
fn parses_artifact_field() {
    let raw = r#"
id = "packed"
artifact = "dist/main.spkx"
"#;
    let m = parse_mod_von(raw).unwrap();
    assert_eq!(m.artifact.as_deref(), Some("dist/main.spkx"));
    assert!(m.entry.is_none());
}

#[test]
fn unknown_field_is_structured() {
    let err = parse_mod_von("id = \"x\"\nfoo = 1\n").unwrap_err();
    assert_eq!(err.code(), "spark.engine.manifest.unknown_field");
    assert!(err.args().get("field").is_some());
    assert!(!err.to_string().contains("未知"));
}
