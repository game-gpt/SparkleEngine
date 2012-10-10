//! Widget 属性从旧值过渡到新值。`Tween` 只是其中一种求值方式。

use spark_core::{Color, Vec2};

use crate::motion::easing::Easing;
use crate::motion::spring::{Spring, SpringParams};
use crate::motion::tween::Tween;

/// 可过渡的 Widget 属性。布局相关属性会在采样变化时标记布局失效。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MotionProperty {
    Opacity,
    Scale,
    TranslateX,
    TranslateY,
    Width,
    Height,
    Padding,
    Margin,
    Progress,
    ScrollOffset,
    /// 颜色 tint。不走布局。
    Color,
}

impl MotionProperty {
    /// 该属性变化是否需要重新布局。
    pub fn affects_layout(self) -> bool {
        matches!(
            self,
            Self::Width | Self::Height | Self::Padding | Self::Margin
        )
    }
}

/// 过渡目标值。属性与值的形状须匹配。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MotionValue {
    Float(f32),
    Vec2(Vec2),
    Color(Color),
}

impl MotionValue {
    pub fn as_float(self) -> Option<f32> {
        match self {
            Self::Float(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_vec2(self) -> Option<Vec2> {
        match self {
            Self::Vec2(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_color(self) -> Option<Color> {
        match self {
            Self::Color(v) => Some(v),
            _ => None,
        }
    }
}

/// 如何到达目标：有限时长缓动，或弹簧。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MotionSpec {
    Tween { duration: f32, easing: Easing },
    Spring(SpringParams),
}

impl MotionSpec {
    pub fn ease_out(duration: f32) -> Self {
        Self::Tween {
            duration,
            easing: Easing::EaseOut,
        }
    }

    pub fn ease_in_out(duration: f32) -> Self {
        Self::Tween {
            duration,
            easing: Easing::EaseInOut,
        }
    }

    pub fn linear(duration: f32) -> Self {
        Self::Tween {
            duration,
            easing: Easing::Linear,
        }
    }

    pub fn spring(params: SpringParams) -> Self {
        Self::Spring(params)
    }

    /// 毫秒便捷构造。
    pub fn ms(duration_ms: u32, easing: Easing) -> Self {
        Self::Tween {
            duration: duration_ms as f32 / 1000.0,
            easing,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Driver {
    FloatTween(Tween<f32>),
    Vec2Tween(Tween<Vec2>),
    ColorTween(Tween<Color>),
    FloatSpring(Spring<f32>),
    Vec2Spring(Spring<Vec2>),
    ColorSpring(Spring<Color>),
}

/// 单个 Widget 属性上的活动过渡。
#[derive(Debug, Clone, PartialEq)]
pub struct Transition {
    pub property: MotionProperty,
    driver: Driver,
}

impl Transition {
    pub fn start(property: MotionProperty, from: MotionValue, to: MotionValue, spec: MotionSpec) -> Option<Self> {
        let driver = match (from, to, spec) {
            (MotionValue::Float(a), MotionValue::Float(b), MotionSpec::Tween { duration, easing }) => {
                Driver::FloatTween(Tween::new(a, b, duration, easing))
            }
            (MotionValue::Vec2(a), MotionValue::Vec2(b), MotionSpec::Tween { duration, easing }) => {
                Driver::Vec2Tween(Tween::new(a, b, duration, easing))
            }
            (MotionValue::Color(a), MotionValue::Color(b), MotionSpec::Tween { duration, easing }) => {
                Driver::ColorTween(Tween::new(a, b, duration, easing))
            }
            (MotionValue::Float(a), MotionValue::Float(b), MotionSpec::Spring(params)) => {
                Driver::FloatSpring(Spring::float(a, b, params))
            }
            (MotionValue::Vec2(a), MotionValue::Vec2(b), MotionSpec::Spring(params)) => {
                Driver::Vec2Spring(Spring::vec2(a, b, params))
            }
            (MotionValue::Color(a), MotionValue::Color(b), MotionSpec::Spring(params)) => {
                Driver::ColorSpring(Spring::color(a, b, params))
            }
            _ => return None,
        };
        Some(Self { property, driver })
    }

    pub fn retarget(&mut self, to: MotionValue) {
        match (&mut self.driver, to) {
            (Driver::FloatTween(tween), MotionValue::Float(v)) => {
                tween.from = tween.sample();
                tween.to = v;
                tween.elapsed = 0.0;
            }
            (Driver::Vec2Tween(tween), MotionValue::Vec2(v)) => {
                tween.from = tween.sample();
                tween.to = v;
                tween.elapsed = 0.0;
            }
            (Driver::ColorTween(tween), MotionValue::Color(v)) => {
                tween.from = tween.sample();
                tween.to = v;
                tween.elapsed = 0.0;
            }
            (Driver::FloatSpring(spring), MotionValue::Float(v)) => spring.target = v,
            (Driver::Vec2Spring(spring), MotionValue::Vec2(v)) => spring.target = v,
            (Driver::ColorSpring(spring), MotionValue::Color(v)) => spring.target = v,
            _ => {}
        }
    }

    pub fn tick(&mut self, dt: f32) -> bool {
        match &mut self.driver {
            Driver::FloatTween(tween) => {
                tween.tick(dt);
                tween.finished()
            }
            Driver::Vec2Tween(tween) => {
                tween.tick(dt);
                tween.finished()
            }
            Driver::ColorTween(tween) => {
                tween.tick(dt);
                tween.finished()
            }
            Driver::FloatSpring(spring) => {
                spring.tick(dt);
                spring.settled()
            }
            Driver::Vec2Spring(spring) => {
                spring.tick(dt);
                spring.settled()
            }
            Driver::ColorSpring(spring) => {
                spring.tick(dt);
                spring.settled()
            }
        }
    }

    pub fn sample(&self) -> MotionValue {
        match &self.driver {
            Driver::FloatTween(tween) => MotionValue::Float(tween.sample()),
            Driver::Vec2Tween(tween) => MotionValue::Vec2(tween.sample()),
            Driver::ColorTween(tween) => MotionValue::Color(tween.sample()),
            Driver::FloatSpring(spring) => MotionValue::Float(spring.value),
            Driver::Vec2Spring(spring) => MotionValue::Vec2(spring.value),
            Driver::ColorSpring(spring) => MotionValue::Color(spring.value),
        }
    }
}
