//! Spark 语言无关 IR：HIR → MIR → LIR、验证、优化与字节码生成。
//!
//! 语言前端完成 AST 语义分析后降低到 HIR。本 crate 不依赖任何语言前端，
//! 也不出现源语言枚举或语言专属节点。`spark-vm` 字节码不是编译器 IR。

#![forbid(missing_docs)]
mod codegen;
mod hir;
mod host;
mod lower;
mod mir;

pub use codegen::{HostEmitMode, emit_module, emit_module_with_host};
pub use hir::{HirBinaryOp, HirExpr, HirFunction, HirModule, HirStmt, HirUnaryOp, PackageId, SymbolId, Ty};
pub use host::{DeterminismKind, HostBindEntry, HostBindTable, HostCompilePolicy, HostEffectKind, HostId, HostPhaseKind};
pub use lower::lower_module;
pub use mir::{BasicBlock, HostRef, IrEffect, MirFunction, MirInst, MirModule, MirTerminator, MirValue};
