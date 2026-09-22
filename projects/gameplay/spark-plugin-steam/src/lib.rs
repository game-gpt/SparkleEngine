//! Steam 脚本插件。
//!
//! 向 `spark-vm` 暴露成就 / 统计 / 云文件 / 用户信息等原生函数。默认内置
//! [`NullSteamBackend`]（内存表）；宿主可换成真实 Steamworks 实现。
//!
//! Rust 游戏逻辑若直接调用 Steam API，请依赖后端 crate，**不必**走本插件。

#![warn(missing_docs)]
mod backend;
mod plugin;
mod runtime;

pub use backend::{NullSteamBackend, SteamBackend};
pub use plugin::{STEAM_NATIVES, SteamPlugin};
pub use runtime::SteamRuntime;
