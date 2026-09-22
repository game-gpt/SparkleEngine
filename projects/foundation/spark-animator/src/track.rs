//! 轨道：通用样本序列，以及骨骼 TRS 曲线。

use spark_geometry::{Quat, Vec3};

use crate::clip::AnimationFrame;

/// 同一属性上的一串帧。
#[derive(Debug, Clone, PartialEq)]
pub struct AnimationTrack<T> {
    /// 轨道名（调试 / 多属性区分），采样不依赖此字段。
    pub name: String,
    /// 按时间升序的关键帧。
    pub frames: Vec<AnimationFrame<T>>,
}

/// 平移或缩放通道上的一个关键帧。
#[derive(Debug, Clone, Copy)]
pub struct Vec3Key {
    /// 相对剪辑起点，单位秒。
    pub time: f32,
    /// 该时刻的三维向量（平移或缩放）。
    pub value: Vec3,
}

/// 旋转通道上的一个关键帧。
#[derive(Debug, Clone, Copy)]
pub struct QuatKey {
    /// 相对剪辑起点，单位秒。
    pub time: f32,
    /// 该时刻的单位四元数（采样侧会再归一化）。
    pub value: Quat,
}

/// 单关节的 TRS 曲线。缺省通道在采样时保持 rest。
#[derive(Debug, Clone, Default)]
pub struct JointTrack {
    /// 目标关节在 [`crate::Skeleton::joints`] 中的下标。
    pub joint: u16,
    /// 局部平移关键；空则沿用 rest 平移。
    pub translations: Vec<Vec3Key>,
    /// 局部旋转关键；空则沿用 rest 旋转。
    pub rotations: Vec<QuatKey>,
    /// 局部缩放关键；空则沿用 rest 缩放。
    pub scales: Vec<Vec3Key>,
}

/// 骨骼剪辑。采样结果是姿态，不是 [`crate::SpriteFrame`]。
#[derive(Debug, Clone)]
pub struct SkinnedAnimationClip {
    /// 剪辑名，供资源侧按名引用。
    pub name: String,
    /// 总时长，单位秒；采样时对时间取模。
    pub duration: f32,
    /// 各关节 TRS 曲线；可只覆盖部分关节。
    pub tracks: Vec<JointTrack>,
}
