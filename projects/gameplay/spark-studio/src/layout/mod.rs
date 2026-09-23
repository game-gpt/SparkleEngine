//! 编辑器停靠布局：尺寸、预设与持久化（不含 Widget 绘制）。

mod persistence;
mod preset;
mod splitter_interaction;

pub use persistence::*;
pub use preset::*;
pub use splitter_interaction::*;
