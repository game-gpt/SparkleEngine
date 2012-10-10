//! 属性插值求值器。不是 UI 子系统，只给 [`super::transition::Transition`] 用。

use spark_core::{Color, Vec2};

use crate::motion::easing::Easing;

/// 一段有限时长的插值。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tween<T> {
    pub from: T,
    pub to: T,
    /// 时长（秒）。
    pub duration: f32,
    pub easing: Easing,
    /// 已过时间（秒）。
    pub elapsed: f32,
}

impl<T> Tween<T> {
    pub fn new(from: T, to: T, duration: f32, easing: Easing) -> Self {
        Self {
            from,
            to,
            duration: duration.max(0.0),
            easing,
            elapsed: 0.0,
        }
    }

    pub fn finished(&self) -> bool {
        self.duration <= 1e-8 || self.elapsed >= self.duration
    }

    pub fn progress(&self) -> f32 {
        if self.duration <= 1e-8 {
            1.0
        } else {
            (self.elapsed / self.duration).clamp(0.0, 1.0)
        }
    }

    /// 推进并返回是否刚结束。
    pub fn tick(&mut self, dt: f32) -> bool {
        if self.finished() {
            return false;
        }
        self.elapsed = (self.elapsed + dt.max(0.0)).min(self.duration.max(0.0));
        self.finished()
    }
}

impl Tween<f32> {
    pub fn sample(&self) -> f32 {
        lerp_f32(self.from, self.to, self.easing.sample(self.progress()))
    }
}

impl Tween<Vec2> {
    pub fn sample(&self) -> Vec2 {
        let t = self.easing.sample(self.progress());
        Vec2::new(
            lerp_f32(self.from.x, self.to.x, t),
            lerp_f32(self.from.y, self.to.y, t),
        )
    }
}

impl Tween<Color> {
    pub fn sample(&self) -> Color {
        let t = self.easing.sample(self.progress());
        Color {
            r: lerp_f32(self.from.r, self.to.r, t),
            g: lerp_f32(self.from.g, self.to.g, t),
            b: lerp_f32(self.from.b, self.to.b, t),
            a: lerp_f32(self.from.a, self.to.a, t),
        }
    }
}

pub(crate) fn lerp_f32(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tween_reaches_end() {
        let mut tween = Tween::new(0.0, 10.0, 0.2, Easing::Linear);
        assert!(!tween.tick(0.1));
        assert!((tween.sample() - 5.0).abs() < 1e-4);
        assert!(tween.tick(0.1));
        assert!((tween.sample() - 10.0).abs() < 1e-4);
    }
}
