//! 自 `src/lib.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_engine_stg::*;
use spark_types::Vec2;

#[test]
fn fan_and_hit() {
    let mut stg = StgEngine::new(".", 256);
    let em =
        Emitter { pattern: EmitPattern::Fan { count: 5, spread_rad: std::f32::consts::FRAC_PI_2, speed: 100.0 }, bullet_radius: 2.0, layer: 0 };
    stg.emit(&em, Vec2::new(0.0, 0.0), Vec2::new(1.0, 0.0));
    assert_eq!(stg.bullets.alive_count(), 5);
    // 子弹打在玩家身上
    let hit = stg.tick(0.0, Vec2::new(0.0, 0.0), 4.0, 8.0);
    assert!(hit);
}

#[test]
fn stage_clock() {
    let mut c = StageClock::default();
    c.advance(0.5);
    assert!((c.time - 0.5).abs() < 1e-5);
    assert!(c.reached(0.4));
    assert!(!c.reached(0.6));
}
