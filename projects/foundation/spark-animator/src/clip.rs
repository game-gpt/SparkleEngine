//! 可采样的动画数据。不含实体当前播到哪一帧。

use crate::track::AnimationTrack;

/// 时间轴上的一个样本。
#[derive(Debug, Clone, PartialEq)]
pub struct AnimationFrame<T> {
    /// 相对剪辑起点的时间，单位秒；帧序列须非递减。
    pub time: f32,
    /// 该时刻的采样值（精灵帧、自定义载荷等）。
    pub value: T,
}

/// 一段可播放的时间数据。`T` 是采样值，例如 [`crate::SpriteFrame`]。
///
/// 轨道按调用方写入的顺序保存。采样默认读第一条轨道。
#[derive(Debug, Clone, PartialEq)]
pub struct AnimationClip<T> {
    /// 剪辑名，供状态机与 [`crate::ClipLibrary`] 按名查找。
    pub name: String,
    /// 总时长，单位秒；须 `> 0` 才有意义可播。
    pub duration: f32,
    /// 属性轨道列表；`sample_clip` 只用下标 0。
    pub tracks: Vec<AnimationTrack<T>>,
}

impl<T> AnimationClip<T> {
    /// 空轨道剪辑。调用方再往 `tracks` 里填数据。
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
