//! 自 `src/lib.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_ecs::*;

#[derive(Debug, PartialEq)]
struct Pos(f32);
#[derive(Debug, PartialEq)]
struct Vel(f32);

#[test]
fn spawn_get_despawn() {
    let mut w = World::new();
    let e = w.spawn(Pos(1.0));
    assert_eq!(w.get::<Pos>(e).unwrap().0, 1.0);
    assert!(w.despawn(e));
    assert!(!w.is_alive(e));
    assert!(w.get::<Pos>(e).is_none());
}

#[test]
fn spawn2_and_foreach() {
    let mut w = World::new();
    let e = w.spawn2(Pos(1.0), Vel(2.0));
    w.for_each2_mut::<Pos, Vel>(|_, p, v| {
        p.0 += v.0;
    });
    assert_eq!(w.get::<Pos>(e).unwrap().0, 3.0);
}

#[test]
fn insert_migrates_archetype() {
    let mut w = World::new();
    let e = w.spawn(Pos(1.0));
    assert!(w.insert(e, Vel(5.0)));
    assert_eq!(w.get::<Vel>(e).unwrap().0, 5.0);
    assert_eq!(w.get::<Pos>(e).unwrap().0, 1.0);
}

#[test]
fn remove_component() {
    let mut w = World::new();
    let e = w.spawn2(Pos(1.0), Vel(2.0));
    let v = w.remove::<Vel>(e).unwrap();
    assert_eq!(v.0, 2.0);
    assert!(w.get::<Vel>(e).is_none());
    assert_eq!(w.get::<Pos>(e).unwrap().0, 1.0);
}

#[test]
fn resources_and_schedule() {
    let mut w = World::new();
    w.resources.insert(7u32);
    let e = w.spawn(Pos(0.0));
    let mut sched = Schedule::new();
    sched.add_fn("bump", |world| {
        let add = *world.resources.get::<u32>().unwrap();
        world.for_each_mut::<Pos>(|_, p| p.0 += add as f32);
    });
    sched.run(&mut w);
    assert_eq!(w.get::<Pos>(e).unwrap().0, 7.0);
}

#[test]
fn generation_rejects_stale() {
    let mut w = World::new();
    let e = w.spawn_empty();
    w.despawn(e);
    let e2 = w.spawn_empty();
    assert_ne!(e, e2);
    assert!(!w.is_alive(e));
    assert!(w.is_alive(e2));
}
