//! Rust 原生游戏插件：注册组件类型、系统与服务（进程内长期存在，不可热卸）。

use super::SparkRuntime;

/// Rust 域游戏包：在引擎启动时装配进 [`SparkRuntime`]。
///
/// 负责权威模拟、高性能子系统与向 Spark Script 暴露的类型 schema（后续接入）。
pub trait NativeGamePlugin {
    /// 向运行时登记资源、Rust 系统、场景与渲染相位。
    fn build(&self, runtime: &mut SparkRuntime);
}
