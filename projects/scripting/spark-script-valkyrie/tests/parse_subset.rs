//! 探针：确认 Oaks `ValkyrieBuilder` 能解析常见语句形态。

#[test]
fn probe_valkyrie_sources() {
    for src in [
        "return 40 + 2",
        "40 + 2",
        "let x = 40",
        "micro f() { return 1 }",
        "micro add(x, y) { return x + y }",
        r#"register_block(1, "a:b", "名", "t.png", 1, 1, 30, "none")"#,
        "",
    ] {
        let r = spark_script_valkyrie::parse(src);
        eprintln!(
            "SRC={src:?} ok={} err={:?}",
            r.is_ok(),
            r.as_ref().err()
        );
        assert!(r.is_ok(), "parse failed for {src:?}: {r:?}");
    }
}
