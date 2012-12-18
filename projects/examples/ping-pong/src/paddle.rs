//! 球拍。

#[derive(Debug, Clone)]
pub struct Paddle {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub speed: f32,
    pub side: PaddleSide,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaddleSide {
    Left,
    Right,
}

impl Paddle {
    pub fn left(court_h: f32) -> Self {
        Self { x: 24.0, y: court_h * 0.5 - 40.0, w: 14.0, h: 80.0, speed: 420.0, side: PaddleSide::Left }
    }

    pub fn right(court_w: f32, court_h: f32) -> Self {
        Self { x: court_w - 38.0, y: court_h * 0.5 - 40.0, w: 14.0, h: 80.0, speed: 420.0, side: PaddleSide::Right }
    }

    pub fn clamp_y(&mut self, court_h: f32) {
        self.y = self.y.clamp(0.0, (court_h - self.h).max(0.0));
    }
}
