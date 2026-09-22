//! 自 `src/lib.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_event::*;

#[derive(Debug, PartialEq)]
struct Boom(u32);

#[test]
fn typed_double_buffer() {
    let mut bus = EventBus::new();
    bus.send(Boom(1));
    bus.send(Boom(2));
    assert_eq!(bus.events::<Boom>().unwrap().len_writing(), 2);
    assert_eq!(bus.events::<Boom>().unwrap().len_reading(), 0);

    bus.update_all();
    let got: Vec<_> = bus.events::<Boom>().unwrap().iter().map(|b| b.0).collect();
    assert_eq!(got, vec![1, 2]);
    assert_eq!(bus.events::<Boom>().unwrap().len_writing(), 0);

    bus.send(Boom(3));
    bus.update_all();
    let got: Vec<_> = bus.events::<Boom>().unwrap().iter().map(|b| b.0).collect();
    assert_eq!(got, vec![3]);
}

#[test]
fn multiple_types() {
    let mut bus = EventBus::new();
    bus.send(Boom(9));
    bus.send("hi".to_string());
    assert_eq!(bus.type_count(), 2);
    bus.update_all();
    assert_eq!(bus.events::<String>().unwrap().iter().next().unwrap(), "hi");
}
