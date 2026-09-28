# spark-animator

Animation clip sampling, playback, and state machines. Supports sprite frames and skinned poses; no drawing, no GPU upload.

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

State machine: `AnimatorController` + `AnimatorState` + `AnimatorTransition` + parameter conditions. Skinning: `Skeleton`, `SkinnedAnimationClip`, `sample_clip` / `evaluate_pose`. ECS side: `AnimatorComponent` / `tick_animators`. UI motion lives in `spark-widget` `motion`.

```bash
cargo test -p spark-animator
```
