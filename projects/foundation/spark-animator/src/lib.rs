//! 动画运行时。
//!
//! [`AnimationClip`] 是可采样的数据。[`AnimationPlayer`] 只记录一段剪辑的进度。
//! [`Animator`] 根据状态、参数和过渡决定播哪一段。
//!
//! 不依赖渲染器。采样结果交给调用方绘制或写入蒙皮。骨骼姿态与精灵帧是两种采样值，不合成一个万能组件。

mod blend;
mod clip;
mod controller;
mod ecs;
mod parameter;
mod player;
mod pose;
mod sampler;
mod skeleton;
mod sprite;
mod state;
mod track;
mod transition;

pub use blend::{add_local_poses, blend_local_poses, blend_masked};
pub use clip::{AnimationClip, AnimationFrame, LoopMode};
pub use controller::{Animator, AnimatorController};
pub use ecs::{
    AnimationDelta, AnimationPlayerComponent, AnimatorComponent, ClipLibrary, install_animator_system, install_player_system, tick_animators,
    tick_players,
};
pub use parameter::ParameterValue;
pub use player::AnimationPlayer;
pub use pose::{LocalPose, build_skin_palette, evaluate_pose, socket_world_matrix, socket_world_position};
pub use sampler::{sample_clip, sample_frames, sample_skinned_clip, wrap_time};
pub use skeleton::{Joint, MAX_JOINTS, Skeleton, Socket};
pub use sprite::SpriteFrame;
pub use state::AnimatorState;
pub use track::{AnimationTrack, JointTrack, QuatKey, SkinnedAnimationClip, Vec3Key};
pub use transition::{AnimatorCondition, AnimatorTransition};

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use spark_core::Rect;
    use spark_ecs::{Schedule, World};
    use spark_geometry::{Quat, Vec3};

    use super::*;

    fn sprite(index: u32) -> SpriteFrame {
        SpriteFrame::new(index, Rect::new(0.0, 0.0, 1.0, 1.0))
    }

    #[test]
    fn sprite_sample_holds_frame_until_next_key() {
        let clip = AnimationClip::single(
            "walk",
            1.0,
            vec![AnimationFrame { time: 0.0, value: sprite(0) }, AnimationFrame { time: 0.5, value: sprite(1) }],
        );
        assert_eq!(sample_clip(&clip, 0.2).unwrap().index, 0);
        assert_eq!(sample_clip(&clip, 0.5).unwrap().index, 1);
    }

    #[test]
    fn player_once_finishes_at_end() {
        let clip = AnimationClip::<SpriteFrame>::single("idle", 1.0, vec![]);
        let mut player = AnimationPlayer::<SpriteFrame>::with_loop(LoopMode::Once);
        player.tick(0.6, clip.duration);
        assert!(!player.finished);
        player.tick(0.6, clip.duration);
        assert!(player.finished);
        assert!((player.time - 1.0).abs() < 1e-5);
        player.pause();
        let frozen = player.time;
        player.tick(1.0, clip.duration);
        assert!((player.time - frozen).abs() < 1e-5);
    }

    #[test]
    fn animator_switches_when_bool_holds() {
        let mut controller = AnimatorController::new("idle");
        controller
            .add_state(AnimatorState::new("idle", "idle"))
            .add_state(AnimatorState::new("run", "run"))
            .add_transition(
                AnimatorTransition::new("idle", "run", 0.2).when(AnimatorCondition::BoolEquals { parameter: "moving".into(), value: true }),
            )
            .set_parameter("moving", ParameterValue::Bool(false));
        let mut animator = Animator::<SpriteFrame>::new(controller);
        let mut durations = HashMap::new();
        durations.insert("idle".into(), 1.0);
        durations.insert("run".into(), 1.0);
        animator.tick(0.1, &durations);
        assert_eq!(animator.current_state(), "idle");
        animator.set_bool("moving", true);
        animator.tick(0.1, &durations);
        assert_eq!(animator.current_state(), "idle");
        assert!(animator.transition_weight() > 0.0);
        animator.tick(0.1, &durations);
        assert_eq!(animator.current_state(), "run");
        assert!(animator.transition_weight().abs() < 1e-5);
    }

    #[test]
    fn ecs_ticks_player_from_library() {
        let mut world = World::new();
        let mut library = ClipLibrary::new();
        library.insert(AnimationClip::single("idle", 1.0, vec![AnimationFrame { time: 0.0, value: sprite(3) }]));
        world.resources.insert(library);
        world.resources.insert(AnimationDelta { dt: 0.25 });
        let entity = world.spawn(AnimationPlayerComponent { clip: "idle".into(), player: AnimationPlayer::<SpriteFrame>::new() });
        let mut schedule = Schedule::new();
        install_player_system::<SpriteFrame>(&mut schedule);
        schedule.run(&mut world);
        let component = world.get::<AnimationPlayerComponent<SpriteFrame>>(entity).unwrap();
        assert!((component.player.time - 0.25).abs() < 1e-5);
    }

    #[test]
    fn two_bone_rest_palette_is_identity_like() {
        let sk = Skeleton::two_bone_chain(1.0);
        let pose = LocalPose::rest(&sk);
        let globals = evaluate_pose(&sk, &pose);
        let palette = build_skin_palette(&sk, &globals);
        assert_eq!(palette.len(), 2);
        for m in &palette {
            for i in 0..16 {
                let expected = if i % 5 == 0 { 1.0 } else { 0.0 };
                assert!((m.cols[i] - expected).abs() < 1e-4, "palette not identity at {i}: {}", m.cols[i]);
            }
        }
    }

    #[test]
    fn socket_follows_parent_joint() {
        let sk = Skeleton::two_bone_chain(1.0);
        let mut pose = LocalPose::rest(&sk);
        pose.locals[0].translation = Vec3::new(0.0, 2.0, 0.0);
        let globals = evaluate_pose(&sk, &pose);
        let world = socket_world_matrix(&sk, &globals, "tip").unwrap();
        let p = world.transform_point(Vec3::ZERO);
        assert!((p.y - 4.0).abs() < 1e-3, "got {}", p.y);
    }

    #[test]
    fn sample_skinned_clip_rotates_root() {
        let sk = Skeleton::two_bone_chain(1.0);
        let clip = SkinnedAnimationClip {
            name: "spin".into(),
            duration: 1.0,
            tracks: vec![JointTrack {
                joint: 0,
                translations: vec![],
                rotations: vec![
                    QuatKey { time: 0.0, value: Quat::IDENTITY },
                    QuatKey { time: 1.0, value: Quat::from_axis_angle(Vec3::Y, std::f32::consts::FRAC_PI_2) },
                ],
                scales: vec![],
            }],
        };
        let pose = sample_skinned_clip(&sk, &clip, 1.0);
        let tip = pose.locals[0].rotation.rotate_vec3(Vec3::X);
        assert!((tip.z + 1.0).abs() < 1e-3, "got {:?}", tip);
    }

    #[test]
    fn additive_layer_offsets_translation() {
        let sk = Skeleton::two_bone_chain(1.0);
        let rest = LocalPose::rest(&sk);
        let mut layer = LocalPose::rest(&sk);
        layer.locals[0].translation = Vec3::new(0.0, 1.0, 0.0);
        let out = add_local_poses(&rest, &layer, &rest, 1.0);
        assert!((out.locals[0].translation.y - 1.0).abs() < 1e-4);
        let half = add_local_poses(&rest, &layer, &rest, 0.5);
        assert!((half.locals[0].translation.y - 0.5).abs() < 1e-4);
    }
}
