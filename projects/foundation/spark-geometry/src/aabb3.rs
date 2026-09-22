//! 三维轴对齐包围盒。

use crate::{Mat4, Vec3};

/// 轴对齐包围盒，半开语义由调用方约定；本实现点包含为闭区间。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Aabb3 {
    /// 各轴最小角点。
    pub min: Vec3,
    /// 各轴最大角点。
    pub max: Vec3,
}

impl Aabb3 {
    /// 由两端点构造，并自动排序使 `min ≤ max` 分量成立。
    pub fn from_min_max(min: Vec3, max: Vec3) -> Self {
        Self {
            min: Vec3::new(min.x.min(max.x), min.y.min(max.y), min.z.min(max.z)),
            max: Vec3::new(min.x.max(max.x), min.y.max(max.y), min.z.max(max.z)),
        }
    }

    /// 由中心与半尺寸（extents）构造：`[center - extents, center + extents]`。
    pub fn from_center_extents(center: Vec3, extents: Vec3) -> Self {
        Self { min: center - extents, max: center + extents }
    }

    /// 单位体素格 `[x,x+1) × [y,y+1) × [z,z+1)`。
    pub fn from_cell(x: i32, y: i32, z: i32) -> Self {
        let min = Vec3::new(x as f32, y as f32, z as f32);
        Self { min, max: min + Vec3::new(1.0, 1.0, 1.0) }
    }

    /// 几何中心。
    pub fn center(self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    /// 半尺寸（中心到各面）。
    pub fn extents(self) -> Vec3 {
        (self.max - self.min) * 0.5
    }

    /// 全尺寸 `max - min`。
    pub fn size(self) -> Vec3 {
        self.max - self.min
    }

    /// 点是否在盒内（各轴闭区间）。
    pub fn contains_point(self, p: Vec3) -> bool {
        p.x >= self.min.x && p.x <= self.max.x && p.y >= self.min.y && p.y <= self.max.y && p.z >= self.min.z && p.z <= self.max.z
    }

    /// 与另一 AABB 是否相交（含面接触）。
    pub fn intersects(self, o: Self) -> bool {
        self.min.x <= o.max.x
            && self.max.x >= o.min.x
            && self.min.y <= o.max.y
            && self.max.y >= o.min.y
            && self.min.z <= o.max.z
            && self.max.z >= o.min.z
    }

    /// 各方向外扩 `margin`（可为负表示内缩）。
    pub fn expand(self, margin: f32) -> Self {
        let m = Vec3::new(margin, margin, margin);
        Self { min: self.min - m, max: self.max + m }
    }

    /// 包围二者的最小 AABB。
    pub fn union(self, o: Self) -> Self {
        Self {
            min: Vec3::new(self.min.x.min(o.min.x), self.min.y.min(o.min.y), self.min.z.min(o.min.z)),
            max: Vec3::new(self.max.x.max(o.max.x), self.max.y.max(o.max.y), self.max.z.max(o.max.z)),
        }
    }

    /// 用模型矩阵变换八个角点后重算 AABB（适合平移/旋转，含缩放）。
    pub fn transformed(self, model: Mat4) -> Self {
        let corners = [
            Vec3::new(self.min.x, self.min.y, self.min.z),
            Vec3::new(self.max.x, self.min.y, self.min.z),
            Vec3::new(self.min.x, self.max.y, self.min.z),
            Vec3::new(self.max.x, self.max.y, self.min.z),
            Vec3::new(self.min.x, self.min.y, self.max.z),
            Vec3::new(self.max.x, self.min.y, self.max.z),
            Vec3::new(self.min.x, self.max.y, self.max.z),
            Vec3::new(self.max.x, self.max.y, self.max.z),
        ];
        let mut min = model.transform_point(corners[0]);
        let mut max = min;
        for c in corners.iter().skip(1) {
            let p = model.transform_point(*c);
            min.x = min.x.min(p.x);
            min.y = min.y.min(p.y);
            min.z = min.z.min(p.z);
            max.x = max.x.max(p.x);
            max.y = max.y.max(p.y);
            max.z = max.z.max(p.z);
        }
        Self { min, max }
    }
}
