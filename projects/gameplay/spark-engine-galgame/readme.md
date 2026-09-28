# spark-engine-galgame

视觉小说 / AVG 骨架：对白、选项、旗标；默认注册 Live2D 脚本插件。

```rust
use spark_engine_galgame::{Choice, DialogueLine, GalgameEngine, ScriptState};

let mut gal = GalgameEngine::new(".");
gal.script.push_line(DialogueLine {
    speaker: Some("A".into()),
    text: "你好".into(),
    voice: None,
});
gal.script.push_choices(vec![
    Choice { label: "去东".into(), set_flag: Some(("route".into(), 1.0)) },
    Choice { label: "去西".into(), set_flag: Some(("route".into(), 2.0)) },
]);
assert!(matches!(gal.advance(), ScriptState::Line(_)));
assert!(matches!(gal.advance(), ScriptState::Choices(_)));
gal.script.choose(0, &mut gal.flags).unwrap();
```

剧本与立绘资源由游戏提供。也可用 `with_live2d_backend`。

```bash
cargo test -p spark-engine-galgame
```
