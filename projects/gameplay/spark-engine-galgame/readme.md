# spark-engine-galgame

Visual novel / AVG skeleton: dialogue, choices, flags; registers the Live2D script plugin by default.

```rust
use spark_engine_galgame::{Choice, DialogueLine, GalgameEngine, ScriptState};

let mut gal = GalgameEngine::new(".");
gal.script.push_line(DialogueLine {
    speaker: Some("A".into()),
    text: "Hello".into(),
    voice: None,
});
gal.script.push_choices(vec![
    Choice { label: "Go east".into(), set_flag: Some(("route".into(), 1.0)) },
    Choice { label: "Go west".into(), set_flag: Some(("route".into(), 2.0)) },
]);
assert!(matches!(gal.advance(), ScriptState::Line(_)));
assert!(matches!(gal.advance(), ScriptState::Choices(_)));
gal.script.choose(0, &mut gal.flags).unwrap();
```

Scripts and portrait assets are provided by the game. Also available: `with_live2d_backend`.

```bash
cargo test -p spark-engine-galgame
```
