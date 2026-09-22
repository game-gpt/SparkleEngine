//! 周期坐标：有限回环空间的规范坐标与最短差（无玩法语义）。

use crate::Vec3;

/// `floor_mod`：结果落在 `[0, size)`；`size <= 0` 时原样返回。
#[inline]
pub fn floor_mod(x: f32, size: f32) -> f32 {
    if !(size > 0.0) {
        return x;
    }
    let mut r = x % size;
    if r < 0.0 {
        r += size;
    }
    // 处理 -0.0 与 size 边界浮点噪声。
    if r >= size { 0.0 } else { r }
}

/// 周期最短有符号差：`target - source` 折到 `(-size/2, size/2]`。
#[inline]
pub fn shortest_delta_1d(source: f32, target: f32, size: f32) -> f32 {
    if !(size > 0.0) {
        return target - source;
    }
    let mut d = target - source;
    d -= (d / size).round() * size;
    d
}

/// 哪些轴启用周期。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PeriodicAxes {
    /// X 轴是否回环。
    pub x: bool,
    /// Y 轴是否回环。
    pub y: bool,
    /// Z 轴是否回环。
    pub z: bool,
}

impl PeriodicAxes {
    /// 三轴均不回环。
    pub const NONE: Self = Self { x: false, y: false, z: false };
    /// 仅 XZ 回环（常见水平环世界）。
    pub const XZ: Self = Self { x: true, y: false, z: true };
    /// 三轴均回环。
    pub const XYZ: Self = Self { x: true, y: true, z: true };
}

/// 各轴周期长度（米或格）；未启用的轴忽略对应分量。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PeriodSize {
    /// X 周期长度；`≤ 0` 时该轴函数视为非周期。
    pub x: f32,
    /// Y 周期长度。
    pub y: f32,
    /// Z 周期长度。
    pub z: f32,
}

impl PeriodSize {
    /// 三轴相同周期长度。
    pub fn uniform(s: f32) -> Self {
        Self { x: s, y: s, z: s }
    }

    /// 仅 XZ 有长度，Y 置 0（配合 [`PeriodicAxes::XZ`]）。
    pub fn xz(sx: f32, sz: f32) -> Self {
        Self { x: sx, y: 0.0, z: sz }
    }
}

/// 将位置规范到各启用轴的 `[0, size)`。
pub fn normalize_position(p: Vec3, axes: PeriodicAxes, size: PeriodSize) -> Vec3 {
    Vec3::new(
        if axes.x { floor_mod(p.x, size.x) } else { p.x },
        if axes.y { floor_mod(p.y, size.y) } else { p.y },
        if axes.z { floor_mod(p.z, size.z) } else { p.z },
    )
}

/// 周期最短位移向量（`target - source`）。
pub fn shortest_delta(source: Vec3, target: Vec3, axes: PeriodicAxes, size: PeriodSize) -> Vec3 {
    Vec3::new(
        if axes.x { shortest_delta_1d(source.x, target.x, size.x) } else { target.x - source.x },
        if axes.y { shortest_delta_1d(source.y, target.y, size.y) } else { target.y - source.y },
        if axes.z { shortest_delta_1d(source.z, target.z, size.z) } else { target.z - source.z },
    )
}

/// 周期最短距离（欧氏长度取最短差向量）。
pub fn periodic_distance(a: Vec3, b: Vec3, axes: PeriodicAxes, size: PeriodSize) -> f32 {
    shortest_delta(a, b, axes, size).length()
}
