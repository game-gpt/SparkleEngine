# spark-animator

动画剪辑采样、播放器与状态机。支持精灵帧与蒙皮姿态；不绘制、不上传 GPU。

```rust
use spark_animator::{AnimationClip, AnimationFrame, SpriteFrame, sample_clip};
use spark_types::Rect;

fn sprite(index: u32) -> SpriteFrame {
    SpriteFrame::new(index, Rect::new(0.0, 0.0, 1.0, 1.0))
}

let clip = AnimationClip::single(
    "walk",
    1.0,
    vec![
        AnimationFrame { time: 0.0, value: sprite(0) },
        AnimationFrame { time: 0.5, value: sprite(1) },
    ],
);
assert_eq!(sample_clip(&clip, 0.2).unwrap().index, 0);
```

状态机：`AnimatorController` + `AnimatorState` + `AnimatorTransition` + 参数条件。蒙皮：`Skeleton`、`SkinnedAnimationClip`、
`sample_clip` / `evaluate_pose`。ECS 侧有 `AnimatorComponent` / `tick_animators`。UI 动效在 `spark-widget` 的 motion。

```bash
cargo test -p spark-animator
```
