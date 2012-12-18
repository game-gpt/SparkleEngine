//! 可采样的动画数据。不含实体当前播到哪一帧。

use crate::track::AnimationTrack;

/// 时间轴上的一个样本。
#[derive(Debug, Clone, PartialEq)]
pub struct AnimationFrame<T> {
    pub time: f32,
    pub value: T,
}

/// 一段可播放的时间数据。`T` 是采样值，例如 [`crate::SpriteFrame`]。
///
/// 轨道按调用方写入的顺序保存。采样默认读第一条轨道。
#[derive(Debug, Clone, PartialEq)]
pub struct AnimationClip<T> {
    pub name: String,
    pub duration: f32,
    pub tracks: Vec<AnimationTrack<T>>,
}

impl<T> AnimationClip<T> {
    pub fn new(name: impl Into<String>, duration: f32) -> Self {
        Self { name: name.into(), duration, tracks: Vec::new() }
    }

    /// 单轨道剪辑。帧时间应由小到大。
    pub fn single(name: impl Into<String>, duration: f32, frames: Vec<AnimationFrame<T>>) -> Self {
        Self { name: name.into(), duration, tracks: vec![AnimationTrack { name: "main".into(), frames }] }
    }
}

/// 循环策略。属于剪辑怎么被播放，不是剪辑里的关键帧。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LoopMode {
    /// 到达终点后从头循环。
    #[default]
    Repeat,
    /// 卡在最后一帧。
    Clamp,
    /// 播完停在终点并标记结束。
    Once,
}
