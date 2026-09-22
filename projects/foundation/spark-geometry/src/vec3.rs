//! 三维向量（渲染 / 体素局部坐标，无游戏语义）。
//!
//! 分量单位由调用方约定（世界米、体素格等）；本类型不做坐标系转换。

/// 三维笛卡尔向量 / 点（同构 `f32` 三元组）。
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vec3 {
    /// X 分量。
    pub x: f32,
    /// Y 分量。
    pub y: f32,
    /// Z 分量。
    pub z: f32,
}

impl Vec3 {
    /// 零向量。
    pub const ZERO: Self = Self { x: 0.0, y: 0.0, z: 0.0 };
    /// 单位 X 轴 `(1,0,0)`。
    pub const X: Self = Self { x: 1.0, y: 0.0, z: 0.0 };
    /// 单位 Y 轴 `(0,1,0)`。
    pub const Y: Self = Self { x: 0.0, y: 1.0, z: 0.0 };
    /// 单位 Z 轴 `(0,0,1)`。
    pub const Z: Self = Self { x: 0.0, y: 0.0, z: 1.0 };

    /// 按分量构造。
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    /// 欧氏长度。
    pub fn length(self) -> f32 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }

    /// 单位化；长度 `≤ 1e-8` 时返回 [`ZERO`]。
    pub fn normalized(self) -> Self {
        let len = self.length();
        if len <= 1e-8 { Self::ZERO } else { Self::new(self.x / len, self.y / len, self.z / len) }
    }

    /// 点积。
    pub fn dot(self, o: Self) -> f32 {
        self.x * o.x + self.y * o.y + self.z * o.z
    }

    /// 叉积（右手系）。
    pub fn cross(self, o: Self) -> Self {
        Self::new(self.y * o.z - self.z * o.y, self.z * o.x - self.x * o.z, self.x * o.y - self.y * o.x)
    }

    /// 转为 `[x, y, z]` 数组（便于上传 GPU）。
    pub fn to_array(self) -> [f32; 3] {
        [self.x, self.y, self.z]
    }
}

impl std::ops::Add for Vec3 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl std::ops::Sub for Vec3 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl std::ops::Mul<f32> for Vec3 {
    type Output = Self;
    fn mul(self, rhs: f32) -> Self {
        Self::new(self.x * rhs, self.y * rhs, self.z * rhs)
    }
}

impl std::ops::Neg for Vec3 {
    type Output = Self;
    fn neg(self) -> Self {
        Self::new(-self.x, -self.y, -self.z)
    }
}
