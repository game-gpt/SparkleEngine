//! Spark 分级日志。可插拔 sink，不含游戏遥测 schema。
//!
//! 宏用法与常见日志库一致：
//! ```ignore
//! info!("ready");
//! info!(target: "ac3", "chunks={}", n);
//! ```

use std::fmt::Write as _;
use std::fs::{File, OpenOptions};
use std::io::Write as IoWrite;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

/// 日志级别（数值越大越严重）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum Level {
    Trace = 0,
    Debug = 1,
    Info = 2,
    Warn = 3,
    Error = 4,
}

impl Level {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Trace => "TRACE",
            Self::Debug => "DEBUG",
            Self::Info => "INFO",
            Self::Warn => "WARN",
            Self::Error => "ERROR",
        }
    }
}

/// 单条日志记录。
#[derive(Debug, Clone)]
pub struct Record {
    pub level: Level,
    pub target: &'static str,
    pub message: String,
}

/// 日志输出端。实现方可写 stderr、文件或环形缓冲。
pub trait LogSink: Send + Sync {
    fn log(&self, record: &Record);
}

fn format_line(record: &Record) -> String {
    format!(
        "[{}] [{}] {}",
        record.level.as_str(),
        record.target,
        record.message
    )
}

/// 默认 stderr sink。
#[derive(Debug, Default)]
pub struct StderrSink;

impl LogSink for StderrSink {
    fn log(&self, record: &Record) {
        eprintln!("{}", format_line(record));
    }
}

/// 追加写入文件（每条立即 flush，便于闪退排查）。
#[derive(Debug)]
pub struct FileSink {
    file: Mutex<File>,
    path: PathBuf,
}

impl FileSink {
    /// 打开或创建日志文件；父目录不存在则创建。
    pub fn open(path: impl AsRef<Path>) -> std::io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)?;
        Ok(Self {
            file: Mutex::new(file),
            path,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl LogSink for FileSink {
    fn log(&self, record: &Record) {
        let line = format_line(record);
        let mut guard = match self.file.lock() {
            Ok(g) => g,
            Err(e) => e.into_inner(),
        };
        let _ = writeln!(guard, "{line}");
        let _ = guard.flush();
    }
}

/// 内存环形 sink（调试与单测用）。
#[derive(Debug, Default)]
pub struct MemorySink {
    inner: Mutex<Vec<Record>>,
    capacity: usize,
}

impl MemorySink {
    pub fn new(capacity: usize) -> Self {
        Self {
            inner: Mutex::new(Vec::new()),
            capacity: capacity.max(1),
        }
    }

    pub fn snapshot(&self) -> Vec<Record> {
        self.inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }
}

impl LogSink for MemorySink {
    fn log(&self, record: &Record) {
        let mut guard = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        if guard.len() >= self.capacity {
            guard.remove(0);
        }
        guard.push(record.clone());
    }
}

/// 引擎日志器：级别过滤 + 多 sink。
#[derive(Clone)]
pub struct Logger {
    min_level: Level,
    sinks: Arc<Vec<Arc<dyn LogSink>>>,
}

impl Logger {
    pub fn builder() -> LoggerBuilder {
        LoggerBuilder::default()
    }

    pub fn log(&self, level: Level, target: &'static str, message: impl Into<String>) {
        if level < self.min_level {
            return;
        }
        let record = Record {
            level,
            target,
            message: message.into(),
        };
        for sink in self.sinks.iter() {
            sink.log(&record);
        }
    }

    pub fn trace(&self, target: &'static str, message: impl Into<String>) {
        self.log(Level::Trace, target, message);
    }

    pub fn debug(&self, target: &'static str, message: impl Into<String>) {
        self.log(Level::Debug, target, message);
    }

    pub fn info(&self, target: &'static str, message: impl Into<String>) {
        self.log(Level::Info, target, message);
    }

    pub fn warn(&self, target: &'static str, message: impl Into<String>) {
        self.log(Level::Warn, target, message);
    }

    pub fn error(&self, target: &'static str, message: impl Into<String>) {
        self.log(Level::Error, target, message);
    }
}

#[derive(Default)]
pub struct LoggerBuilder {
    min_level: Option<Level>,
    sinks: Vec<Arc<dyn LogSink>>,
}

impl LoggerBuilder {
    pub fn min_level(mut self, level: Level) -> Self {
        self.min_level = Some(level);
        self
    }

    pub fn sink(mut self, sink: Arc<dyn LogSink>) -> Self {
        self.sinks.push(sink);
        self
    }

    pub fn stderr(self) -> Self {
        self.sink(Arc::new(StderrSink))
    }

    /// 追加文件 sink；打开失败则跳过（仍可用其它 sink）。
    pub fn file(self, path: impl AsRef<Path>) -> Self {
        match FileSink::open(path) {
            Ok(sink) => self.sink(Arc::new(sink)),
            Err(_) => self,
        }
    }

    pub fn build(self) -> Logger {
        let sinks = if self.sinks.is_empty() {
            vec![Arc::new(StderrSink) as Arc<dyn LogSink>]
        } else {
            self.sinks
        };
        Logger {
            min_level: self.min_level.unwrap_or(Level::Info),
            sinks: Arc::new(sinks),
        }
    }
}

static GLOBAL: OnceLock<Logger> = OnceLock::new();

/// 安装进程级默认日志器。重复调用忽略后续安装。
pub fn install_global(logger: Logger) -> bool {
    GLOBAL.set(logger).is_ok()
}

/// 取进程级日志器；未安装则惰性创建 stderr Info 级默认器。
pub fn global() -> &'static Logger {
    GLOBAL.get_or_init(|| Logger::builder().stderr().min_level(Level::Info).build())
}

/// 格式化辅助：把 `format_args!` 结果收成 `String`。
pub fn format_msg(args: std::fmt::Arguments<'_>) -> String {
    let mut buf = String::new();
    let _ = buf.write_fmt(args);
    buf
}

/// 安装 stderr + 可选文件，并挂 panic 钩子把 panic 写入同一通道。
pub fn install_std(file: Option<&Path>, min_level: Level) -> bool {
    let mut b = Logger::builder().min_level(min_level).stderr();
    if let Some(path) = file {
        b = b.file(path);
    }
    let ok = install_global(b.build());
    if ok {
        let default_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let msg = if let Some(s) = info.payload().downcast_ref::<&str>() {
                (*s).to_string()
            } else if let Some(s) = info.payload().downcast_ref::<String>() {
                s.clone()
            } else {
                "Box<Any>".into()
            };
            let loc = info
                .location()
                .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
                .unwrap_or_else(|| "?".into());
            global().error("panic", format!("{msg} @ {loc}"));
            default_hook(info);
        }));
    }
    ok
}

#[macro_export]
macro_rules! log {
    ($level:expr, target: $target:expr, $($arg:tt)*) => {{
        $crate::global().log($level, $target, $crate::format_msg(format_args!($($arg)*)));
    }};
    ($level:expr, $($arg:tt)*) => {{
        $crate::global().log($level, module_path!(), $crate::format_msg(format_args!($($arg)*)));
    }};
}

#[macro_export]
macro_rules! trace {
    (target: $target:expr, $($arg:tt)*) => {
        $crate::log!($crate::Level::Trace, target: $target, $($arg)*)
    };
    ($($arg:tt)*) => {
        $crate::log!($crate::Level::Trace, $($arg)*)
    };
}

#[macro_export]
macro_rules! debug {
    (target: $target:expr, $($arg:tt)*) => {
        $crate::log!($crate::Level::Debug, target: $target, $($arg)*)
    };
    ($($arg:tt)*) => {
        $crate::log!($crate::Level::Debug, $($arg)*)
    };
}

#[macro_export]
macro_rules! info {
    (target: $target:expr, $($arg:tt)*) => {
        $crate::log!($crate::Level::Info, target: $target, $($arg)*)
    };
    ($($arg:tt)*) => {
        $crate::log!($crate::Level::Info, $($arg)*)
    };
}

#[macro_export]
macro_rules! warn {
    (target: $target:expr, $($arg:tt)*) => {
        $crate::log!($crate::Level::Warn, target: $target, $($arg)*)
    };
    ($($arg:tt)*) => {
        $crate::log!($crate::Level::Warn, $($arg)*)
    };
}

#[macro_export]
macro_rules! error {
    (target: $target:expr, $($arg:tt)*) => {
        $crate::log!($crate::Level::Error, target: $target, $($arg)*)
    };
    ($($arg:tt)*) => {
        $crate::log!($crate::Level::Error, $($arg)*)
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_sink_filters_by_level() {
        let mem = Arc::new(MemorySink::new(16));
        let logger = Logger::builder()
            .min_level(Level::Warn)
            .sink(mem.clone())
            .build();
        logger.info("t", "skip");
        logger.warn("t", "keep");
        let snap = mem.snapshot();
        assert_eq!(snap.len(), 1);
        assert_eq!(snap[0].level, Level::Warn);
        assert_eq!(snap[0].message, "keep");
    }

    #[test]
    fn file_sink_appends() {
        let dir = std::env::temp_dir().join("spark_logger_test");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("t.log");
        let _ = std::fs::remove_file(&path);
        let sink = FileSink::open(&path).expect("open");
        sink.log(&Record {
            level: Level::Info,
            target: "test",
            message: "hello".into(),
        });
        let text = std::fs::read_to_string(&path).expect("read");
        assert!(text.contains("hello"));
    }
}
