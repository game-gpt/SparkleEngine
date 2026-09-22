//! Spark **Edit Runtime**：编辑自动化的权威执行面。
//!
//! CLI（`spark shell`）、MCP、Studio 与 CI 应调用本 crate，而不是各自实现资源写入。
//! 当前可执行格式为 VON 编辑计划（`EditPlan`）；Sparkle Script Edit profile 将绑定同一
//! [`EditSession`]。

#![warn(missing_docs)]

mod diagnostic;
mod op;
mod plan;
mod report;
mod session;

pub use diagnostic::{Diagnostic, Severity};
pub use op::EditOp;
pub use plan::EditPlan;
pub use report::{ChangeKind, ChangeRecord, EditReport, TransactionState};
pub use session::{EditMode, EditSession};
