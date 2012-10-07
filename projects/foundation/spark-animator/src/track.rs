//! 轨道：通用样本序列，以及骨骼 TRS 曲线。

use spark_geometry::{Quat, Vec3};

use crate::clip::AnimationFrame;

/// 同一属性上的一串帧。
#[derive(Debug, Clone, PartialEq)]
pub struct AnimationTrack<T> {
    pub name: String,
    pub frames: Vec<AnimationFrame<T>>,
}

#[derive(Debug, Clone, Copy)]
pub struct Vec3Key {
    pub time: f32,
    pub value: Vec3,
}

#[derive(Debug, Clone, Copy)]
pub struct QuatKey {
    pub time: f32,
    pub value: Quat,
}

/// 单关节的 TRS 曲线。缺省通道在采样时保持 rest。
#[derive(Debug, Clone, Default)]
pub struct JointTrack {
    pub joint: u16,
    pub translations: Vec<Vec3Key>,
    pub rotations: Vec<QuatKey>,
    pub scales: Vec<Vec3Key>,
}

/// 骨骼剪辑。采样结果是姿态，不是 [`crate::SpriteFrame`]。
#[derive(Debug, Clone)]
pub struct SkinnedAnimationClip {
    pub name: String,
    pub duration: f32,
    pub tracks: Vec<JointTrack>,
}
