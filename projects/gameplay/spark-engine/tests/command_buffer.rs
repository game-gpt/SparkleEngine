//! 自 `src/command_buffer.rs` 迁出的原 `#[cfg(test)] mod tests`。
use spark_engine::*;

#[test]
fn drain_clears_buffer() {
    let mut buf = ScriptCommandBuffer::new();
    buf.spawn("unit");
    buf.despawn(1);
    assert_eq!(buf.len(), 2);
    let cmds = buf.drain();
    assert_eq!(cmds.len(), 2);
    assert!(buf.is_empty());
    assert!(matches!(cmds[0], ScriptCommand::Spawn { .. }));
}
