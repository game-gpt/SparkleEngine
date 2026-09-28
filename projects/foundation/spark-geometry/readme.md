# spark-geometry

2D/3D geometry and transforms: `Circle`, `Ray`, `Aabb3`, `Mat4` / `Quat` / `Trs`, rectangle AABB intersection, and more.

```rust
use spark_types::{Rect, Vec2};
use spark_geometry::{Circle, Ray, aabb_aabb, circle_circle, ray_circle};

let a = Circle::new(Vec2::new(0.0, 0.0), 1.0);
let b = Circle::new(Vec2::new(1.5, 0.0), 1.0);
assert!(circle_circle(a, b));

let ray = Ray::new(Vec2::new(-5.0, 0.0), Vec2::new(1.0, 0.0));
let t = ray_circle(ray, a).unwrap();
assert!((t - 4.0).abs() < 1e-4);

assert!(aabb_aabb(Rect::new(0.0, 0.0, 2.0, 2.0), Rect::new(1.0, 1.0, 2.0, 2.0)));
```

More 3D / quaternion / TRS coverage in `tests/`. `spark-physics` and `spark-renderer` depend on this crate.

```bash
cargo test -p spark-geometry
```
