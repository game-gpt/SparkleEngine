//! 弹簧：朝目标逼近，适合按下回弹与悬停反馈。

use spark_core::{Color, Vec2};

/// 一维或向量弹簧参数。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpringParams {
    pub stiffness: f32,
    pub damping: f32,
    /// 位置误差与速度都低于此值时视为静止。
    pub rest_epsilon: f32,
}

impl Default for SpringParams {
    fn default() -> Self {
        Self {
            stiffness: 180.0,
            damping: 20.0,
            rest_epsilon: 0.001,
        }
    }
}

/// 朝 `target` 运动的弹簧状态。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spring<T> {
    pub value: T,
    pub velocity: T,
    pub target: T,
    pub params: SpringParams,
}

impl Spring<f32> {
    pub fn float(value: f32, target: f32, params: SpringParams) -> Self {
        Self {
            value,
            velocity: 0.0,
            target,
            params,
        }
    }

    pub fn settled(&self) -> bool {
        (self.value - self.target).abs() < self.params.rest_epsilon
            && self.velocity.abs() < self.params.rest_epsilon
    }

    pub fn tick(&mut self, dt: f32) {
        let dt = dt.max(0.0);
        let force = (self.target - self.value) * self.params.stiffness
            - self.velocity * self.params.damping;
        self.velocity += force * dt;
        self.value += self.velocity * dt;
        if self.settled() {
            self.value = self.target;
            self.velocity = 0.0;
        }
    }
}

impl Spring<Vec2> {
    pub fn vec2(value: Vec2, target: Vec2, params: SpringParams) -> Self {
        Self {
            value,
            velocity: Vec2::ZERO,
            target,
            params,
        }
    }

    pub fn settled(&self) -> bool {
        let eps = self.params.rest_epsilon;
        (self.value.x - self.target.x).abs() < eps
            && (self.value.y - self.target.y).abs() < eps
            && self.velocity.x.abs() < eps
            && self.velocity.y.abs() < eps
    }

    pub fn tick(&mut self, dt: f32) {
        let dt = dt.max(0.0);
        let fx = (self.target.x - self.value.x) * self.params.stiffness
            - self.velocity.x * self.params.damping;
        let fy = (self.target.y - self.value.y) * self.params.stiffness
            - self.velocity.y * self.params.damping;
        self.velocity.x += fx * dt;
        self.velocity.y += fy * dt;
        self.value.x += self.velocity.x * dt;
        self.value.y += self.velocity.y * dt;
        if self.settled() {
            self.value = self.target;
            self.velocity = Vec2::ZERO;
        }
    }
}

impl Spring<Color> {
    pub fn color(value: Color, target: Color, params: SpringParams) -> Self {
        Self {
            value,
            velocity: Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.0,
            },
            target,
            params,
        }
    }

    pub fn settled(&self) -> bool {
        let eps = self.params.rest_epsilon;
        channel_near(self.value.r, self.target.r, self.velocity.r, eps)
            && channel_near(self.value.g, self.target.g, self.velocity.g, eps)
            && channel_near(self.value.b, self.target.b, self.velocity.b, eps)
            && channel_near(self.value.a, self.target.a, self.velocity.a, eps)
    }

    pub fn tick(&mut self, dt: f32) {
        let dt = dt.max(0.0);
        tick_channel(
            &mut self.value.r,
            &mut self.velocity.r,
            self.target.r,
            self.params,
            dt,
        );
        tick_channel(
            &mut self.value.g,
            &mut self.velocity.g,
            self.target.g,
            self.params,
            dt,
        );
        tick_channel(
            &mut self.value.b,
            &mut self.velocity.b,
            self.target.b,
            self.params,
            dt,
        );
        tick_channel(
            &mut self.value.a,
            &mut self.velocity.a,
            self.target.a,
            self.params,
            dt,
        );
        if self.settled() {
            self.value = self.target;
            self.velocity = Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.0,
            };
        }
    }
}

fn channel_near(value: f32, target: f32, velocity: f32, eps: f32) -> bool {
    (value - target).abs() < eps && velocity.abs() < eps
}

fn tick_channel(value: &mut f32, velocity: &mut f32, target: f32, params: SpringParams, dt: f32) {
    let force = (target - *value) * params.stiffness - *velocity * params.damping;
    *velocity += force * dt;
    *value += *velocity * dt;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spring_settles_on_target() {
        let mut spring = Spring::float(0.0, 1.0, SpringParams::default());
        for _ in 0..120 {
            spring.tick(1.0 / 60.0);
        }
        assert!(spring.settled());
        assert!((spring.value - 1.0).abs() < 1e-3);
    }
}
