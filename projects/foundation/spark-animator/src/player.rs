//! 单段播放进度。不决定下一段播什么。

use std::marker::PhantomData;

use crate::{
    clip::{AnimationClip, LoopMode},
    sampler::sample_clip,
};

/// 一个剪辑的播放时钟。`T` 只标记这段剪辑的采样值类型。
#[derive(Debug, Clone)]
pub struct AnimationPlayer<T> {
    pub time: f32,
    pub speed: f32,
    pub loop_mode: LoopMode,
    pub playing: bool,
    pub finished: bool,
    _sample: PhantomData<T>,
}

impl<T> Default for AnimationPlayer<T> {
    fn default() -> Self {
        Self { time: 0.0, speed: 1.0, loop_mode: LoopMode::Repeat, playing: true, finished: false, _sample: PhantomData }
    }
}

impl<T> AnimationPlayer<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_loop(mode: LoopMode) -> Self {
        Self { loop_mode: mode, ..Self::default() }
    }

    pub fn play(&mut self) {
        self.playing = true;
        self.finished = false;
    }

    pub fn pause(&mut self) {
        self.playing = false;
    }

    pub fn stop(&mut self) {
        self.playing = false;
        self.time = 0.0;
        self.finished = false;
    }

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

    pub fn sample(&self, clip: &AnimationClip<T>) -> Option<T>
    where
        T: Clone,
    {
        sample_clip(clip, self.time)
    }
}
