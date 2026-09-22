//! 自 `src/event.rs` 迁出的原 `#[cfg(test)] mod tests`。
use std::sync::Arc;

use spark_diagnostics::ErrorArg;
use spark_logger::*;

#[test]
fn code_line_keeps_stable_event() {
    let ev = LogEvent::new(Level::Error, "media", "spark.media.open_failed").field("path", ErrorArg::Path(Arc::from("clips/intro.bin")));
    let line = ev.code_line();
    assert!(line.contains("event=spark.media.open_failed"));
    assert!(line.contains("path=clips/intro.bin"));
    assert!(!line.contains("无法"));
}
