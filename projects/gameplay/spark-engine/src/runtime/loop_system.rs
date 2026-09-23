//! Rust 域 [`LoopSystem`]：带 [`SystemContext`] 与 [`RustCommands`] 的相位系统。

use super::SystemContext;

/// Rust 原生相位系统（游戏逻辑优先实现本 trait，而非 `FnMut(&mut World)`）。
pub trait LoopSystem: Send {
    /// 调试与调度日志用名称。
    fn name(&self) -> &'static str;

    /// 在受控上下文中执行一帧逻辑。
    fn run(&mut self, ctx: &mut SystemContext<'_>);
}

/// 函数式 [`LoopSystem`] 包装。
pub struct LoopSystemFn<F> {
    name: &'static str,
    f: F,
}

impl<F> LoopSystemFn<F>
where
    F: FnMut(&mut SystemContext<'_>) + Send,
{
    /// 用静态名称与闭包构造。
    pub fn new(name: &'static str, f: F) -> Self {
        Self { name, f }
    }
}

impl<F> LoopSystem for LoopSystemFn<F>
where
    F: FnMut(&mut SystemContext<'_>) + Send,
{
    fn name(&self) -> &'static str {
        self.name
    }

    fn run(&mut self, ctx: &mut SystemContext<'_>) {
        (self.f)(ctx);
    }
}
