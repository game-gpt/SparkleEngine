//! 自 `src/lib.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_engine_galgame::*;

#[test]
fn dialogue_and_choice() {
    let mut gal = GalgameEngine::new(".");
    gal.script.push_line(DialogueLine { speaker: Some("A".into()), text: "你好".into(), voice: None });
    gal.script.push_choices(vec![
        Choice { label: "去东".into(), set_flag: Some(("route".into(), 1.0)) },
        Choice { label: "去西".into(), set_flag: Some(("route".into(), 2.0)) },
    ]);
    assert!(matches!(gal.advance(), ScriptState::Line(_)));
    assert!(matches!(gal.advance(), ScriptState::Choices(_)));
    gal.script.choose(0, &mut gal.flags).unwrap();
    assert_eq!(gal.flags.get("route"), 1.0);
    assert!(matches!(gal.advance(), ScriptState::Ended));
}

#[test]
fn live2d_plugin_registered_by_default() {
    let gal = GalgameEngine::new(".");
    assert!(gal.engine.plugins().contains("live2d"));
    assert!(gal.engine.plugins().native_names().iter().any(|n| *n == "live2d_load"));
    let id = gal.live2d().borrow_mut().backend.load("demo.model3.json").unwrap();
    gal.live2d().borrow_mut().backend.set_param(id, "ParamMouthOpenY", 0.8).unwrap();
    let v = gal.live2d().borrow().backend.get_param(id, "ParamMouthOpenY").unwrap();
    assert!((v - 0.8).abs() < 1e-5);
}
