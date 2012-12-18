//! 简单多边形（顶点顺时针或逆时针均可；点内检测用绕数）。

use spark_core::Vec2;

#[derive(Debug, Clone, PartialEq)]
pub struct Polygon {
    pub vertices: Vec<Vec2>,
}

impl Polygon {
    pub fn new(vertices: Vec<Vec2>) -> Self {
        Self { vertices }
    }

    pub fn len(&self) -> usize {
        self.vertices.len()
    }

    pub fn is_empty(&self) -> bool {
        self.vertices.is_empty()
    }

    /// 偶数绕数规则点在多边形内（含边界近似）。
    pub fn contains(&self, p: Vec2) -> bool {
        let n = self.vertices.len();
        if n < 3 {
            return false;
        }
        let mut inside = false;
        let mut j = n - 1;
        for i in 0..n {
            let vi = self.vertices[i];
            let vj = self.vertices[j];
            let intersect = ((vi.y > p.y) != (vj.y > p.y)) && (p.x < (vj.x - vi.x) * (p.y - vi.y) / (vj.y - vi.y + 1e-12) + vi.x);
            if intersect {
                inside = !inside;
            }
            j = i;
        }
        inside
    }
}
