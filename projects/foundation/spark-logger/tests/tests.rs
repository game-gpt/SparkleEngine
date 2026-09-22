//! 自 `src/lib.rs` 迁出的原 `#[cfg(test)] mod tests`。
use std::sync::Arc;

use spark_diagnostics::ErrorArg;
use spark_logger::*;

#[test]
fn memory_sink_filters_by_level() {
    let mem = Arc::new(MemorySink::new(16));
    let logger = Logger::builder().min_level(Level::Warn).sink(mem.clone()).build();
    logger.info("t", "skip");
    logger.warn("t", "keep");
    let snap = mem.snapshot();
    assert_eq!(snap.len(), 1);
    assert_eq!(snap[0].level, Level::Warn);
    assert_eq!(snap[0].message, "keep");
}

#[test]
fn structured_event_code_line() {
    let ev = LogEvent::new(Level::Error, "media", "spark.media.open_failed").field("io_kind", ErrorArg::String(Arc::from("not_found")));
    let line = ev.code_line();
    assert!(line.contains("event=spark.media.open_failed"));
    assert!(line.contains("io_kind=not_found"));
}

#[test]
fn file_sink_appends() {
    let dir = std::env::temp_dir().join("spark_logger_test");
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("t.log");
    let _ = std::fs::remove_file(&path);
    let sink = FileSink::open(&path).expect("open");
    sink.log(&Record { level: Level::Info, target: "test", message: "hello".into() });
    let text = std::fs::read_to_string(&path).expect("read");
    assert!(text.contains("hello"));
}
