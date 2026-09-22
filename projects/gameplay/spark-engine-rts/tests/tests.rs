//! 自 `src/lib.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_engine_rts::*;

use spark_types::Vec2;

#[test]
fn select_and_move() {
    let mut rts = RtsEngine::new(".", 32, 32, 1.0);
    let p = PlayerId(1);
    let a = rts.roster.spawn(p, UnitPose::at(2.0, 2.0), 4.0);
    let b = rts.roster.spawn(p, UnitPose::at(10.0, 10.0), 4.0);
    rts.box_select(Vec2::new(0.0, 0.0), Vec2::new(5.0, 5.0), Some(p));
    assert_eq!(rts.selection.ids(), &[a]);
    rts.commands.issue(a, Command::MoveTo { target: Vec2::new(8.0, 2.0), speed: 4.0 });
    rts.tick(1.0, p);
    let u = rts.roster.get(a).unwrap();
    assert!((u.pose.pos.x - 6.0).abs() < 1e-3);
    assert!(rts.fog.is_visible(2, 2));
    let _ = b;
}
