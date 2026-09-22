# spark-geometry

2D/3D 几何与变换：`Circle`、`Ray`、`Aabb3`、`Mat4` / `Quat` / `Trs`、矩形 AABB 相交等。

```rust
use spark_core::{Rect, Vec2};
use spark_geometry::{Circle, Ray, aabb_aabb, circle_circle, ray_circle};

let a = Circle::new(Vec2::new(0.0, 0.0), 1.0);
let b = Circle::new(Vec2::new(1.5, 0.0), 1.0);
assert!(circle_circle(a, b));

let ray = Ray::new(Vec2::new(-5.0, 0.0), Vec2::new(1.0, 0.0));
let t = ray_circle(ray, a).unwrap();
assert!((t - 4.0).abs() < 1e-4);

assert!(aabb_aabb(Rect::new(0.0, 0.0, 2.0, 2.0), Rect::new(1.0, 1.0, 2.0, 2.0)));
```

更多 3D / 四元数 / TRS 见 `tests/`。`spark-physics` 与 `spark-renderer` 依赖本 crate。

```bash
cargo test -p spark-geometry
```
