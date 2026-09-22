//! 球。

/// 球场上的球：圆心与半径为逻辑像素，速度为像素/秒。
#[derive(Debug, Clone)]
pub struct Ball {
    /// 圆心 X（像素）。
    pub x: f32,
    /// 圆心 Y（像素）。
    pub y: f32,
    /// 水平速度（像素/秒）；正值向右。
    pub vx: f32,
    /// 竖直速度（像素/秒）；正值向下。
    pub vy: f32,
    /// 碰撞/绘制半径（像素）。发球默认 8。
    pub radius: f32,
    /// 基准速率（像素/秒）；反弹时用当前速率与此值取较大者再加速。
    pub speed: f32,
}

impl Ball {
    /// 从球场中心发球：基准速 360；`to_right` 决定 `vx` 符号，`vy` 初值为 `speed * 0.35`。
    pub fn serve(court_w: f32, court_h: f32, to_right: bool) -> Self {
        let speed = 360.0;
        Self { x: court_w * 0.5, y: court_h * 0.5, vx: if to_right { speed } else { -speed }, vy: speed * 0.35, radius: 8.0, speed }
    }
}
