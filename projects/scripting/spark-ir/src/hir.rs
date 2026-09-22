//! Spark HIR：统一语义层（非 VM 指令）。

use std::sync::Arc;

use spark_diagnostics::SourceSpan;

use crate::host::HostId;

/// 包身份（IR 层轻量表示）。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PackageId {
    /// 包名（逻辑名，非磁盘路径）。
    pub name: Arc<str>,
    /// 版本字符串（前端约定，IR 不解析 semver）。
    pub version: Arc<str>,
}

impl PackageId {
    /// 构造具名包身份。
    pub fn new(name: impl Into<Arc<str>>, version: impl Into<Arc<str>>) -> Self {
        Self { name: name.into(), version: version.into() }
    }

    /// 匿名占位包（单文件 / 测试前端常用）。
    pub fn anonymous() -> Self {
        Self::new("_anonymous", "0")
    }
}

/// 稳定符号身份（链接用，不是显示名）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SymbolId {
    /// 包在编译单元内的下标。
    pub package_index: u32,
    /// 模块在包内的下标。
    pub module_index: u32,
    /// 符号在模块内的下标。
    pub local_index: u32,
}

/// 公共值类型（跨语言前端统一）。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Ty {
    /// 空值。
    Null,
    /// 布尔。
    Bool,
    /// 整数（宽度由后端约定，HIR 不钉死位数）。
    Int,
    /// 浮点。
    Float,
    /// 字符串。
    String,
    /// ECS 实体句柄。
    Entity,
    /// 逻辑资产键。
    Asset,
    /// 不透明资源句柄。
    ResourceHandle,
    /// 同质数组。
    Array(Box<Ty>),
    /// 映射。
    Map {
        /// 键类型。
        key: Box<Ty>,
        /// 值类型。
        value: Box<Ty>,
    },
    /// 结构化记录（字段在别处描述）。
    Record,
    /// 一等函数值。
    Function,
    /// 协程 / 可挂起计算。
    Coroutine,
    /// 动态类型（运行时再分辨）。
    Dynamic,
    /// 前端自定义不透明类型名。
    Opaque(Arc<str>),
}

/// HIR 表达式。
#[derive(Debug, Clone, PartialEq)]
pub enum HirExpr {
    /// `null` 字面量。
    LiteralNull {
        /// 源码跨度（可选）。
        span: Option<SourceSpan>,
    },
    /// 布尔字面量。
    LiteralBool {
        /// 字面值。
        value: bool,
        /// 源码跨度（可选）。
        span: Option<SourceSpan>,
    },
    /// 数字字面量（整数与浮点在 HIR 统一为 `f64` 槽，类型由上下文收窄）。
    LiteralNumber {
        /// 数值。
        value: f64,
        /// 源码跨度（可选）。
        span: Option<SourceSpan>,
    },
    /// 字符串字面量。
    LiteralString {
        /// 字符串内容。
        value: Arc<str>,
        /// 源码跨度（可选）。
        span: Option<SourceSpan>,
    },
    /// 读取局部槽。
    Local {
        /// 局部下标（相对当前函数 `locals` / 形参布局）。
        index: u32,
        /// 源码跨度（可选）。
        span: Option<SourceSpan>,
    },
    /// 已解析的函数下标（模块内）。
    FuncRef {
        /// 模块内函数下标。
        func_index: u32,
        /// 源码跨度（可选）。
        span: Option<SourceSpan>,
    },
    /// 调用：`callee` 求值为函数后再传参。
    Call {
        /// 被调表达式。
        callee: Box<HirExpr>,
        /// 实参列表。
        args: Vec<HirExpr>,
        /// 源码跨度（可选）。
        span: Option<SourceSpan>,
    },
    /// 调试打印（非宿主槽位；对应 VM `Print`）。
    Print {
        /// 打印值。
        value: Box<HirExpr>,
        /// 源码跨度（可选）。
        span: Option<SourceSpan>,
    },
    /// 动态方法派发（接收者 + 方法名 + 实参）。
    DynamicSend {
        /// 接收者。
        receiver: Box<HirExpr>,
        /// 方法名（运行时解析）。
        method: Arc<str>,
        /// 实参。
        args: Vec<HirExpr>,
        /// 源码跨度（可选）。
        span: Option<SourceSpan>,
    },
    /// 宿主调用（已解析 [`HostId`]）。
    HostCall {
        /// 宿主函数身份。
        host: HostId,
        /// 实参。
        args: Vec<HirExpr>,
        /// 来自绑定表的效果快照（供 MIR 汇总）。
        effects: Vec<crate::host::HostEffectKind>,
        /// 源码跨度（可选）。
        span: Option<SourceSpan>,
    },
    /// 二元运算。
    Binary {
        /// 运算符。
        op: HirBinaryOp,
        /// 左操作数。
        lhs: Box<HirExpr>,
        /// 右操作数。
        rhs: Box<HirExpr>,
        /// 源码跨度（可选）。
        span: Option<SourceSpan>,
    },
    /// 一元运算。
    Unary {
        /// 运算符。
        op: HirUnaryOp,
        /// 操作数。
        expr: Box<HirExpr>,
        /// 源码跨度（可选）。
        span: Option<SourceSpan>,
    },
    /// 转为布尔（条件位置的真值转换）。
    ToBool {
        /// 源表达式。
        expr: Box<HirExpr>,
        /// 源码跨度（可选）。
        span: Option<SourceSpan>,
    },
    /// 表达式形式的 `if`（两分支均产出值）。
    If {
        /// 条件。
        cond: Box<HirExpr>,
        /// then 分支值。
        then_branch: Box<HirExpr>,
        /// else 分支值。
        else_branch: Box<HirExpr>,
        /// 源码跨度（可选）。
        span: Option<SourceSpan>,
    },
    /// 块：先执行语句，再取可选结果表达式。
    Block {
        /// 块内语句。
        stmts: Vec<HirStmt>,
        /// 块结果；`None` 表示结果为 `null`/unit。
        result: Option<Box<HirExpr>>,
        /// 源码跨度（可选）。
        span: Option<SourceSpan>,
    },
}

/// HIR 二元运算符。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HirBinaryOp {
    /// 加。
    Add,
    /// 减。
    Sub,
    /// 乘。
    Mul,
    /// 除。
    Div,
    /// 取模。
    Mod,
    /// 相等。
    Eq,
    /// 不等。
    Ne,
    /// 小于。
    Lt,
    /// 小于等于。
    Le,
    /// 大于。
    Gt,
    /// 大于等于。
    Ge,
}

/// HIR 一元运算符。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HirUnaryOp {
    /// 数值取负。
    Neg,
    /// 逻辑非。
    Not,
}

/// HIR 语句。
#[derive(Debug, Clone, PartialEq)]
pub enum HirStmt {
    /// 纯表达式语句（丢弃结果）。
    Expr {
        /// 表达式。
        expr: HirExpr,
        /// 源码跨度（可选）。
        span: Option<SourceSpan>,
    },
    /// 写入局部槽。
    AssignLocal {
        /// 局部下标。
        index: u32,
        /// 右值。
        value: HirExpr,
        /// 源码跨度（可选）。
        span: Option<SourceSpan>,
    },
    /// 从当前函数返回。
    Return {
        /// 返回值；`None` 表示无返回值。
        value: Option<HirExpr>,
        /// 源码跨度（可选）。
        span: Option<SourceSpan>,
    },
    /// 语句形式的 `if`。
    If {
        /// 条件。
        cond: HirExpr,
        /// then 体。
        then_body: Vec<HirStmt>,
        /// else 体（可空）。
        else_body: Vec<HirStmt>,
        /// 源码跨度（可选）。
        span: Option<SourceSpan>,
    },
    /// `while` 循环。
    While {
        /// 条件。
        cond: HirExpr,
        /// 循环体。
        body: Vec<HirStmt>,
        /// 源码跨度（可选）。
        span: Option<SourceSpan>,
    },
    /// 跳出最近一层 `While`。
    Break {
        /// 源码跨度（可选）。
        span: Option<SourceSpan>,
    },
}

/// HIR 函数。
#[derive(Debug, Clone, PartialEq)]
pub struct HirFunction {
    /// 显示名。
    pub name: Arc<str>,
    /// 可选稳定符号（跨模块链接）。
    pub symbol: Option<SymbolId>,
    /// 形参：`(名字, 类型)`，顺序即调用约定。
    pub params: Vec<(Arc<str>, Ty)>,
    /// 返回类型。
    pub return_ty: Ty,
    /// 局部变量表（含形参之后的槽）。
    pub locals: Vec<(Arc<str>, Ty)>,
    /// 函数体语句序列。
    pub body: Vec<HirStmt>,
    /// 源码跨度（可选）。
    pub span: Option<SourceSpan>,
}

/// HIR 模块。
#[derive(Debug, Clone, PartialEq)]
pub struct HirModule {
    /// 所属包。
    pub package: PackageId,
    /// 模块名。
    pub name: Arc<str>,
    /// 模块内函数列表（下标即 [`HirExpr::FuncRef`]）。
    pub functions: Vec<HirFunction>,
}
