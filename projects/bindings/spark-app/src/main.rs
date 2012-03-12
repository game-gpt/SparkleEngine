//! Spark 演示入口二进制 `spark`。

use spark_ecs::World;
use spark_event::EventBus;
use spark_time::Time;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let mut world = World::new();
    let _e = world.spawn_empty();
    let mut time = Time::default();
    time.advance(1.0 / 60.0);
    let mut events = EventBus::new();
    events.push("boot");
    let _ = world.tick_placeholder();

    tracing::info!(
        entities = world.entity_count_hint(),
        events = events.len(),
        "spark 脚手架就绪"
    );
    println!("spark: 元引擎脚手架就绪。");
}
