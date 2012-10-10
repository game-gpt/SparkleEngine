//! 缓动曲线。只把归一化时间映射到 [0, 1]，不含属性语义。

/// 时间曲线。输入与输出都是 `[0, 1]`。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Easing {
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
    /// 三次贝塞尔控制点 `(x1, y1, x2, y2)`，端点固定为 `(0,0)` 与 `(1,1)`。
    CubicBezier([f32; 4]),
    /// 阶跃：未到终点为 0，到达为 1。
    Step,
}

impl Default for Easing {
    fn default() -> Self {
        Self::EaseOut
    }
}

impl Easing {
    pub fn sample(self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Self::Linear => t,
            Self::EaseIn => t * t,
            Self::EaseOut => 1.0 - (1.0 - t) * (1.0 - t),
            Self::EaseInOut => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
                }
            }
            Self::CubicBezier(points) => cubic_bezier(points, t),
            Self::Step => {
                if t >= 1.0 {
                    1.0
                } else {
                    0.0
                }
            }
        }
    }
}

fn cubic_bezier(points: [f32; 4], t: f32) -> f32 {
    // 用牛顿迭代求参数 u，使 Bezier_x(u) ≈ t，再取 Bezier_y(u)。
    let [x1, y1, x2, y2] = points;
    let mut u = t;
    for _ in 0..6 {
        let x = bezier(u, x1, x2);
        let dx = bezier_dx(u, x1, x2);
        if dx.abs() < 1e-6 {
            break;
        }
        u = (u - (x - t) / dx).clamp(0.0, 1.0);
    }
    bezier(u, y1, y2)
}

fn bezier(u: f32, p1: f32, p2: f32) -> f32 {
    let c = 3.0 * p1;
    let b = 3.0 * (p2 - p1) - c;
    let a = 1.0 - c - b;
    ((a * u + b) * u + c) * u
}

fn bezier_dx(u: f32, p1: f32, p2: f32) -> f32 {
    let c = 3.0 * p1;
    let b = 3.0 * (p2 - p1) - c;
    let a = 1.0 - c - b;
    (3.0 * a * u + 2.0 * b) * u + c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ease_out_starts_fast() {
        let mid = Easing::EaseOut.sample(0.25);
        assert!(mid > 0.4);
        assert!((Easing::Linear.sample(0.5) - 0.5).abs() < 1e-5);
        assert!((Easing::Step.sample(0.99)).abs() < 1e-5);
        assert!((Easing::Step.sample(1.0) - 1.0).abs() < 1e-5);
    }
}
