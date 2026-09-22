//! Spark **Edit Runtime**：编辑自动化的权威执行面。
//!
//! CLI（`spark shell`）、MCP、Studio 与 CI 应调用本 crate，而不是各自实现资源写入。
//! 当前可执行格式为 VON 编辑计划（`EditPlan`）；Sparkle Script Edit profile 将绑定同一
//! [`EditSession`]。

#![warn(missing_docs)]

mod capabilities;
mod diagnostic;
mod host_api;
mod op;
mod parse;
mod plan;
mod report;
mod session;

pub use capabilities::EditCapabilities;
pub use diagnostic::{Diagnostic, Severity};
pub use host_api::{EDIT_PROFILE_ID, EditHostScript, HostCall, lower_host_call, names as host_names};
pub use op::EditOp;
pub use parse::parse_edit_source;
pub use plan::EditPlan;
pub use report::{ChangeKind, ChangeRecord, EditReport, TransactionState};
pub use session::{EditMode, EditSession};

use std::path::Path;

/// 解析 VON（`ops` 或 Edit profile `calls`）并执行。
pub fn run_edit_plan(root: impl AsRef<Path>, mode: EditMode, von: &str) -> Result<EditReport, String> {
    run_edit_plan_with(root, mode, von, EditCapabilities::default())
}

/// 带能力门闩的执行入口（MCP 应用 `EditCapabilities::read_only()`）。
pub fn run_edit_plan_with(
    root: impl AsRef<Path>,
    mode: EditMode,
    von: &str,
    caps: EditCapabilities,
) -> Result<EditReport, String> {
    let plan = parse_edit_source(von)?;
    let mut session = EditSession::new(root.as_ref(), mode).with_capabilities(caps);
    Ok(session.run(&plan))
}
