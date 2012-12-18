//! 弹簧配置占位。

#[derive(Debug, Clone, Copy)]
pub struct SpringConfig {
    pub stiffness: f32,
    pub damping: f32,
    pub mass: f32,
}

impl SpringConfig {
    pub fn snappy() -> Self {
        Self { stiffness: 300.0, damping: 20.0, mass: 1.0 }
    }
}

impl Default for SpringConfig {
    fn default() -> Self {
        Self::snappy()
    }
}
