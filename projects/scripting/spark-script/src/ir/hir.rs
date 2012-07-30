//! Spark HIR：统一语义层（非 VM 指令）。
//!
//! 语言糖应在进入 HIR 前由前端展开。HIR 仍可保留源映射以便诊断。

use std::sync::Arc;

use spark_diagnostics::SourceSpan;

use crate::request::PackageId;

/// 稳定符号身份（链接用，不是显示名）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SymbolId {
    pub package_index: u32,
    pub module_index: u32,
    pub local_index: u32,
}

/// 公共值类型（动态语言可大量使用 [`Ty::Dynamic`]）。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Ty {
    Null,
    Bool,
    Int,
    Float,
    String,
    Entity,
    Asset,
    ResourceHandle,
    Array(Box<Ty>),
    Map {
        key: Box<Ty>,
        value: Box<Ty>,
    },
    Record,
    Function,
    Coroutine,
    Dynamic,
    Opaque(Arc<str>),
}

/// HIR 表达式。
#[derive(Debug, Clone, PartialEq)]
pub enum HirExpr {
    LiteralNull {
        span: Option<SourceSpan>,
    },
    LiteralBool {
        value: bool,
        span: Option<SourceSpan>,
    },
    LiteralNumber {
        value: f64,
        span: Option<SourceSpan>,
    },
    LiteralString {
        value: Arc<str>,
        span: Option<SourceSpan>,
    },
    Local {
        index: u32,
        span: Option<SourceSpan>,
    },
    /// 语义已明确的调用（参数个数与 ABI 在检查阶段校验）。
    Call {
        callee: Box<HirExpr>,
        args: Vec<HirExpr>,
        span: Option<SourceSpan>,
    },
    /// 规则固定的动态发送（效果可见，不是未定义的语言专用节点）。
    DynamicSend {
        receiver: Box<HirExpr>,
        method: Arc<str>,
        args: Vec<HirExpr>,
        span: Option<SourceSpan>,
    },
    /// 宿主槽位调用（降低后应使用链接槽，此处可先保留 stable id 字符串）。
    HostCall {
        host_name: Arc<str>,
        args: Vec<HirExpr>,
        span: Option<SourceSpan>,
    },
    Binary {
        op: HirBinaryOp,
        lhs: Box<HirExpr>,
        rhs: Box<HirExpr>,
        span: Option<SourceSpan>,
    },
    Unary {
        op: HirUnaryOp,
        expr: Box<HirExpr>,
        span: Option<SourceSpan>,
    },
    /// 显式真值转换（各语言规则已在前端展开到此）。
    ToBool {
        expr: Box<HirExpr>,
        span: Option<SourceSpan>,
    },
    If {
        cond: Box<HirExpr>,
        then_branch: Box<HirExpr>,
        else_branch: Box<HirExpr>,
        span: Option<SourceSpan>,
    },
    Block {
        stmts: Vec<HirStmt>,
        result: Option<Box<HirExpr>>,
        span: Option<SourceSpan>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HirBinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HirUnaryOp {
    Neg,
    Not,
}

/// HIR 语句。
#[derive(Debug, Clone, PartialEq)]
pub enum HirStmt {
    Expr {
        expr: HirExpr,
        span: Option<SourceSpan>,
    },
    AssignLocal {
        index: u32,
        value: HirExpr,
        span: Option<SourceSpan>,
    },
    Return {
        value: Option<HirExpr>,
        span: Option<SourceSpan>,
    },
    /// 显式分支（短路等已展开为此）。
    If {
        cond: HirExpr,
        then_body: Vec<HirStmt>,
        else_body: Vec<HirStmt>,
        span: Option<SourceSpan>,
    },
    While {
        cond: HirExpr,
        body: Vec<HirStmt>,
        span: Option<SourceSpan>,
    },
}

/// HIR 函数。
#[derive(Debug, Clone, PartialEq)]
pub struct HirFunction {
    pub name: Arc<str>,
    pub symbol: Option<SymbolId>,
    pub params: Vec<(Arc<str>, Ty)>,
    pub return_ty: Ty,
    pub locals: Vec<(Arc<str>, Ty)>,
    pub body: Vec<HirStmt>,
    pub span: Option<SourceSpan>,
}

/// HIR 模块。
#[derive(Debug, Clone, PartialEq)]
pub struct HirModule {
    pub package: PackageId,
    pub name: Arc<str>,
    pub functions: Vec<HirFunction>,
}
