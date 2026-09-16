//! Valkyrie 风格子集 AST（`micro` / `let` / 表达式）。

#[derive(Debug, Clone)]
pub struct ValkyrieRoot {
    pub items: Vec<Item>,
}

#[derive(Debug, Clone)]
pub enum Item {
    Micro(Micro),
    Stmt(Stmt),
}

#[derive(Debug, Clone)]
pub struct Micro {
    pub name: String,
    pub params: Vec<String>,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Let { name: String, value: Expr },
    Expr(Expr),
    Return(Option<Expr>),
}

#[derive(Debug, Clone)]
pub enum Expr {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Name(String),
    Unary { op: UnaryOp, expr: Box<Expr> },
    Binary {
        op: BinOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    Call { name: String, args: Vec<Expr> },
    If {
        cond: Box<Expr>,
        then_body: Vec<Stmt>,
        else_body: Option<Vec<Stmt>>,
    },
    While {
        cond: Box<Expr>,
        body: Vec<Stmt>,
    },
    Block(Vec<Stmt>),
}

#[derive(Debug, Clone, Copy)]
pub enum UnaryOp {
    Neg,
    Not,
}

#[derive(Debug, Clone, Copy)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
}
