//! 球。

#[derive(Debug, Clone)]
pub struct Ball {
    pub speed: f32,
    pub radius: f32,
}

impl Default for Ball {
    fn default() -> Self {
        Self {
            speed: 360.0,
            radius: 8.0,
        }
    }
}
