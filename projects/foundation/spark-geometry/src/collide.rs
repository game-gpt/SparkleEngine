//! 相交与最近点。

use spark_core::{Rect, Vec2};

use crate::{
    Vec2Ext,
    circle::Circle,
    line::{LineSegment, Ray},
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClosestPoint {
    pub point: Vec2,
    pub distance: f32,
}

pub fn point_in_circle(p: Vec2, c: Circle) -> bool {
    c.contains(p)
}

pub fn point_in_aabb(p: Vec2, r: Rect) -> bool {
    r.contains(p)
}

pub fn circle_circle(a: Circle, b: Circle) -> bool {
    let r = a.radius + b.radius;
    a.center.distance_squared(b.center) <= r * r
}

pub fn aabb_aabb(a: Rect, b: Rect) -> bool {
    a.x < b.x + b.w && a.x + a.w > b.x && a.y < b.y + b.h && a.y + a.h > b.y
}

pub fn circle_aabb(c: Circle, r: Rect) -> bool {
    let cx = c.center.x.clamp(r.x, r.x + r.w);
    let cy = c.center.y.clamp(r.y, r.y + r.h);
    let dx = c.center.x - cx;
    let dy = c.center.y - cy;
    dx * dx + dy * dy <= c.radius * c.radius
}

/// 射线–圆：返回沿射线参数 `t`（`origin + t * dir`），`dir` 非单位时 `t` 按实际 dir 度量。
pub fn ray_circle(ray: Ray, c: Circle) -> Option<f32> {
    let f = ray.origin.sub(c.center);
    let a = ray.dir.length_squared();
    if a < 1e-12 {
        return None;
    }
    let b = 2.0 * f.dot(ray.dir);
    let cc = f.length_squared() - c.radius * c.radius;
    let disc = b * b - 4.0 * a * cc;
    if disc < 0.0 {
        return None;
    }
    let s = disc.sqrt();
    let t0 = (-b - s) / (2.0 * a);
    let t1 = (-b + s) / (2.0 * a);
    if t0 >= 0.0 {
        Some(t0)
    }
    else if t1 >= 0.0 {
        Some(t1)
    }
    else {
        None
    }
}

/// 线段相交（含端点接触）。
pub fn segment_segment(a: LineSegment, b: LineSegment) -> bool {
    let d1 = a.delta();
    let d2 = b.delta();
    let cross = d1.cross(d2);
    if cross.abs() < 1e-8 {
        // 共线：投影重叠粗测
        return colinear_overlap(a, b);
    }
    let t = b.a.sub(a.a).cross(d2) / cross;
    let u = b.a.sub(a.a).cross(d1) / cross;
    (0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u)
}

fn colinear_overlap(a: LineSegment, b: LineSegment) -> bool {
    let d = a.delta();
    let len2 = d.length_squared();
    if len2 < 1e-12 {
        return b.closest_point(a.a).distance_squared(a.a) < 1e-8;
    }
    let ta0 = 0.0;
    let ta1 = 1.0;
    let tb0 = b.a.sub(a.a).dot(d) / len2;
    let tb1 = b.b.sub(a.a).dot(d) / len2;
    let (tb_min, tb_max) = if tb0 < tb1 { (tb0, tb1) } else { (tb1, tb0) };
    tb_max >= ta0 - 1e-6 && tb_min <= ta1 + 1e-6
}
