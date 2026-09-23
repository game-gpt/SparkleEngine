//! 编辑器停靠布局：尺寸、预设与持久化（不含 Widget 绘制）。

mod persistence;
mod preset;
mod splitter_interaction;
mod storage;
mod viewport_rect;

pub use persistence::*;
pub use preset::*;
pub use splitter_interaction::*;
pub use storage::*;
pub use viewport_rect::*;
