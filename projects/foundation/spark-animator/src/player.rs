//! 单段播放进度。不决定下一段播什么。

use std::marker::PhantomData;

use crate::{
    clip::{AnimationClip, LoopMode},
    sampler::sample_clip,
};

/// 一个剪辑的播放时钟。`T` 只标记这段剪辑的采样值类型。
#[derive(Debug, Clone)]
pub struct AnimationPlayer<T> {
    /// 当前本地时间，单位秒；由 [`Self::tick`] 按循环策略折叠。
    pub time: f32,
    /// 时间倍率；`1.0` 为实时，可为负以倒放。
    pub speed: f32,
    /// 到达终点后的行为。
    pub loop_mode: LoopMode,
    /// `false` 时 `tick` 不再推进。
    pub playing: bool,
    /// [`LoopMode::Once`] 播完后为 `true`，直至 `play` / `stop` / `reset`。
    pub finished: bool,
    _sample: PhantomData<T>,
}

impl<T> Default for AnimationPlayer<T> {
    fn default() -> Self {
        Self { time: 0.0, speed: 1.0, loop_mode: LoopMode::Repeat, playing: true, finished: false, _sample: PhantomData }
    }
}

impl<T> AnimationPlayer<T> {
    /// 默认：正在播、`Repeat`、速度 1。
    pub fn new() -> Self {
        Self::default()
    }

    /// 指定循环策略的播放器。
    pub fn with_loop(mode: LoopMode) -> Self {
        Self { loop_mode: mode, ..Self::default() }
    }

    /// 继续推进，并清除 `finished`。
    pub fn play(&mut self) {
        self.playing = true;
        self.finished = false;
    }

    /// 暂停时间推进，保留当前 `time`。
    pub fn pause(&mut self) {
        self.playing = false;
    }

    /// 停到起点：`time = 0`，清除 `finished`。
    pub fn stop(&mut self) {
        self.playing = false;
        self.time = 0.0;
        self.finished = false;
    }

    /// 时间归零并清除 `finished`，不改变 `playing`。
    pub fn reset(&mut self) {
        self.time = 0.0;
        self.finished = false;
    }

    /// 按 `dt` 推进。暂停或已经播完则不动。
    pub fn tick(&mut self, dt: f32, duration: f32) {
        if !self.playing || self.finished || duration <= 1e-8 {
            return;
        }
        self.time += dt * self.speed;
        match self.loop_mode {
            LoopMode::Repeat => {
                while self.time >= duration {
                    self.time -= duration;
                }
                while self.time < 0.0 {
                    self.time += duration;
                }
            }
            LoopMode::Clamp => {
                self.time = self.time.clamp(0.0, duration);
            }
            LoopMode::Once => {
                if self.time >= duration {
                    self.time = duration;
                    self.finished = true;
                }
                else if self.time < 0.0 {
                    self.time = 0.0;
                }
            }
        }
    }

    /// 用当前 `time` 对剪辑做阶梯采样（读第一条轨道）。
    pub fn sample(&self, clip: &AnimationClip<T>) -> Option<T>
    where
        T: Clone,
    {
        sample_clip(clip, self.time)
    }
}
