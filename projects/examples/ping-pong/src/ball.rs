//! 球。

#[derive(Debug, Clone)]
pub struct Ball {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub radius: f32,
    pub speed: f32,
}

impl Ball {
    pub fn serve(court_w: f32, court_h: f32, to_right: bool) -> Self {
        let speed = 360.0;
        Self { x: court_w * 0.5, y: court_h * 0.5, vx: if to_right { speed } else { -speed }, vy: speed * 0.35, radius: 8.0, speed }
    }
}
