//! 自 `src/light2d.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_types::*;

#[test]
fn sky_stops_at_blocker_and_light_spreads_through_open_cells() {
    let mut grid = LightGrid2d::filled(0, 0, 3, 3, LightRgb::new(0.0, 0.0, 0.0));
    grid.paint_sky_columns(LightRgb::new(1.0, 1.0, 1.0), |x, y| x == 1 && y == 1);
    assert!(grid.get(0, 2).r > 0.9);
    assert!(grid.get(1, 0).r > 0.9);
    assert!(grid.get(1, 1).r < 0.01);
    assert!(grid.get(1, 2).r < 0.01);

    let mut spread = LightGrid2d::filled(0, 0, 2, 2, LightRgb::new(0.0, 0.0, 0.0));
    spread.set_max(0, 0, LightRgb::new(1.0, 0.0, 0.0));
    spread.propagate(LightFalloff { open: 0.1, blocked: 0.45, bump: 0.001, cutoff: 0.05 }, |x, y| x == 1 && y == 0);
    assert!(spread.get(0, 1).r > spread.get(1, 0).r);
    assert!(spread.get(0, 1).r > 0.8);
}
