//! Spark HIR：统一语义层（非 VM 指令）。

use std::sync::Arc;

use spark_diagnostics::SourceSpan;

/// 包身份（IR 层轻量表示）。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PackageId {
    pub name: Arc<str>,
    pub version: Arc<str>,
}

impl PackageId {
    pub fn new(name: impl Into<Arc<str>>, version: impl Into<Arc<str>>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
        }
    }

    pub fn anonymous() -> Self {
        Self::new("_anonymous", "0")
    }
}

/// 稳定符号身份（链接用，不是显示名）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SymbolId {
    pub package_index: u32,
    pub module_index: u32,
    pub local_index: u32,
}

/// 公共值类型。
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
    /// 已解析的函数下标（模块内）。
    FuncRef {
        func_index: u32,
        span: Option<SourceSpan>,
    },
    Call {
        callee: Box<HirExpr>,
        args: Vec<HirExpr>,
        span: Option<SourceSpan>,
    },
    /// 调试打印（非宿主槽位；对应 VM `Print`）。
    Print {
        value: Box<HirExpr>,
        span: Option<SourceSpan>,
    },
    DynamicSend {
        receiver: Box<HirExpr>,
        method: Arc<str>,
        args: Vec<HirExpr>,
        span: Option<SourceSpan>,
    },
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
