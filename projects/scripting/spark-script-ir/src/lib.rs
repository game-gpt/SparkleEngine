//! Spark 脚本公共 IR：HIR / MIR、降低与字节码生成。
//!
//! 语言前端将语义明确后的构造降到 HIR。共享优化与字节码生成只认本 crate。
//! `spark-vm` 字节码不是公共编译器 IR。

mod codegen;
mod hir;
mod host;
mod lower;
mod mir;

pub use codegen::{emit_module, emit_module_with_host, HostEmitMode};
pub use hir::{
    HirBinaryOp, HirExpr, HirFunction, HirModule, HirStmt, HirUnaryOp, PackageId, SymbolId, Ty,
};
pub use host::{HostBindEntry, HostBindTable, HostId};
pub use lower::lower_module;
pub use mir::{
    BasicBlock, HostRef, IrEffect, MirFunction, MirInst, MirModule, MirTerminator, MirValue,
};
