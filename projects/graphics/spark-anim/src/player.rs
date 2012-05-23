//! 剪辑播放时钟（采样仍走 `sample_clip`）。

use crate::clip::{sample_clip, AnimationClip};
use crate::pose::LocalPose;
use crate::skeleton::Skeleton;

/// 循环策略。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LoopMode {
    /// 到达终点后从头循环。
    #[default]
    Repeat,
    /// 卡在最后一帧。
    Clamp,
    /// 播完停在终点并标记 `finished`。
    Once,
}

/// 轻量剪辑播放器：推进时间并采样局部姿态。
#[derive(Debug, Clone)]
pub struct AnimationPlayer {
    pub time: f32,
    pub speed: f32,
    pub looping: LoopMode,
    pub finished: bool,
}

impl Default for AnimationPlayer {
    fn default() -> Self {
        Self {
            time: 0.0,
            speed: 1.0,
            looping: LoopMode::Repeat,
            finished: false,
        }
    }
}

impl AnimationPlayer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_loop(mode: LoopMode) -> Self {
        Self {
            looping: mode,
            ..Self::default()
        }
    }

    /// 按 `dt` 推进；`duration` 来自当前剪辑。
    pub fn tick(&mut self, dt: f32, duration: f32) {
        if self.finished || duration <= 1e-8 {
            return;
        }
        self.time += dt * self.speed;
        match self.looping {
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
                } else if self.time < 0.0 {
                    self.time = 0.0;
                }
            }
        }
    }

    pub fn reset(&mut self) {
        self.time = 0.0;
        self.finished = false;
    }

    pub fn sample(&self, skeleton: &Skeleton, clip: &AnimationClip) -> LocalPose {
        sample_clip(skeleton, clip, self.time)
    }
}
