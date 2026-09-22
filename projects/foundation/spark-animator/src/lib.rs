//! 动画运行时。
//!
//! [`AnimationClip`] 是可采样的数据。[`AnimationPlayer`] 只记录一段剪辑的进度。
//! [`Animator`] 根据状态、参数和过渡决定播哪一段。
//!
//! 不依赖渲染器。采样结果交给调用方绘制或写入蒙皮。骨骼姿态与精灵帧是两种采样值，不合成一个万能组件。

#![forbid(missing_docs)]
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
