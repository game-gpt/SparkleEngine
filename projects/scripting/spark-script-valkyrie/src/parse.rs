//! Valkyrie 子集递归下降。
//!
//! 上游 `oak-valkyrie` Builder 对表达式语句还原仍不稳定，本前端用手写子集
//! 覆盖 `micro` / `let` / `return` / `if` / `while` / 调用，保证归一到 `spark-vm`。

use crate::ast::{BinOp, Expr, Item, Micro, Stmt, UnaryOp, ValkyrieRoot};

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Ident(String),
    Number(f64),
    String(String),
    KwMicro,
    KwLet,
    KwReturn,
    KwIf,
    KwElse,
    KwWhile,
    KwLoop,
    KwTrue,
    KwFalse,
    KwNull,
    KwAnd,
    KwOr,
    Plus,
    Minus,
    Star,
    Slash,
    EqEq,
    NotEq,
    Lt,
    Le,
    Gt,
    Ge,
    Assign,
    LParen,
    RParen,
    LBrace,
    RBrace,
    Comma,
    Bang,
    Eof,
}

pub fn parse(source: &str) -> Result<ValkyrieRoot, String> {
    let tokens = lex(source)?;
    let mut p = Parser { tokens, i: 0 };
    p.parse_root()
}

struct Parser {
    tokens: Vec<Tok>,
    i: usize,
}

impl Parser {
    fn peek(&self) -> &Tok {
        self.tokens.get(self.i).unwrap_or(&Tok::Eof)
    }

    fn bump(&mut self) -> Tok {
        let t = self.tokens.get(self.i).cloned().unwrap_or(Tok::Eof);
        if self.i < self.tokens.len() {
            self.i += 1;
        }
        t
    }

    fn eat(&mut self, expect: &Tok) -> Result<(), String> {
        let got = self.bump();
        if &got == expect {
            Ok(())
        } else {
            Err(format!("expected_token:{expect:?}:got:{got:?}"))
        }
    }

    fn parse_root(&mut self) -> Result<ValkyrieRoot, String> {
        let mut items = Vec::new();
        while !matches!(self.peek(), Tok::Eof) {
            if matches!(self.peek(), Tok::KwMicro) {
                items.push(Item::Micro(self.parse_micro()?));
            } else {
                items.push(Item::Stmt(self.parse_stmt()?));
            }
        }
        Ok(ValkyrieRoot { items })
    }

    fn parse_micro(&mut self) -> Result<Micro, String> {
        self.eat(&Tok::KwMicro)?;
        // 兼容 legacy `fn`
        let name = match self.bump() {
            Tok::Ident(n) => n,
            other => return Err(format!("expected_micro_name:{other:?}")),
        };
        self.eat(&Tok::LParen)?;
        let mut params = Vec::new();
        if !matches!(self.peek(), Tok::RParen) {
            loop {
                match self.bump() {
                    Tok::Ident(p) => params.push(p),
                    other => return Err(format!("expected_param_name:{other:?}")),
                }
                if matches!(self.peek(), Tok::Comma) {
                    self.bump();
                    continue;
                }
                break;
            }
        }
        self.eat(&Tok::RParen)?;
        let body = self.parse_block_stmts()?;
        Ok(Micro { name, params, body })
    }

    fn parse_block_stmts(&mut self) -> Result<Vec<Stmt>, String> {
        self.eat(&Tok::LBrace)?;
        let mut body = Vec::new();
        while !matches!(self.peek(), Tok::RBrace | Tok::Eof) {
            body.push(self.parse_stmt()?);
        }
        self.eat(&Tok::RBrace)?;
        Ok(body)
    }

    fn parse_stmt(&mut self) -> Result<Stmt, String> {
        match self.peek() {
            Tok::KwLet => {
                self.bump();
                let name = match self.bump() {
                    Tok::Ident(n) => n,
                    other => return Err(format!("expected_binding_name:{other:?}")),
                };
                self.eat(&Tok::Assign)?;
                let value = self.parse_expr()?;
                Ok(Stmt::Let { name, value })
            }
            Tok::KwReturn => {
                self.bump();
                if matches!(
                    self.peek(),
                    Tok::RBrace | Tok::Eof | Tok::KwMicro | Tok::KwLet | Tok::KwIf | Tok::KwWhile | Tok::KwLoop
                ) {
                    Ok(Stmt::Return(None))
                } else {
                    Ok(Stmt::Return(Some(self.parse_expr()?)))
                }
            }
            _ => Ok(Stmt::Expr(self.parse_expr()?)),
        }
    }

    fn parse_expr(&mut self) -> Result<Expr, String> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Result<Expr, String> {
        let mut lhs = self.parse_and()?;
        while matches!(self.peek(), Tok::KwOr) {
            self.bump();
            let rhs = self.parse_and()?;
            lhs = Expr::Binary {
                op: BinOp::Or,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }
        Ok(lhs)
    }

    fn parse_and(&mut self) -> Result<Expr, String> {
        let mut lhs = self.parse_cmp()?;
        while matches!(self.peek(), Tok::KwAnd) {
            self.bump();
            let rhs = self.parse_cmp()?;
            lhs = Expr::Binary {
                op: BinOp::And,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }
        Ok(lhs)
    }

    fn parse_cmp(&mut self) -> Result<Expr, String> {
        let mut lhs = self.parse_add()?;
        let op = match self.peek() {
            Tok::EqEq => Some(BinOp::Eq),
            Tok::NotEq => Some(BinOp::Ne),
            Tok::Lt => Some(BinOp::Lt),
            Tok::Le => Some(BinOp::Le),
            Tok::Gt => Some(BinOp::Gt),
            Tok::Ge => Some(BinOp::Ge),
            _ => None,
        };
        if let Some(op) = op {
            self.bump();
            let rhs = self.parse_add()?;
            lhs = Expr::Binary {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }
        Ok(lhs)
    }

    fn parse_add(&mut self) -> Result<Expr, String> {
        let mut lhs = self.parse_mul()?;
        loop {
            let op = match self.peek() {
                Tok::Plus => Some(BinOp::Add),
                Tok::Minus => Some(BinOp::Sub),
                _ => None,
            };
            let Some(op) = op else { break };
            self.bump();
            let rhs = self.parse_mul()?;
            lhs = Expr::Binary {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }
        Ok(lhs)
    }

    fn parse_mul(&mut self) -> Result<Expr, String> {
        let mut lhs = self.parse_unary()?;
        loop {
            let op = match self.peek() {
                Tok::Star => Some(BinOp::Mul),
                Tok::Slash => Some(BinOp::Div),
                _ => None,
            };
            let Some(op) = op else { break };
            self.bump();
            let rhs = self.parse_unary()?;
            lhs = Expr::Binary {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }
        Ok(lhs)
    }

    fn parse_unary(&mut self) -> Result<Expr, String> {
        match self.peek() {
            Tok::Minus => {
                self.bump();
                Ok(Expr::Unary {
                    op: UnaryOp::Neg,
                    expr: Box::new(self.parse_unary()?),
                })
            }
            Tok::Bang => {
                self.bump();
                Ok(Expr::Unary {
                    op: UnaryOp::Not,
                    expr: Box::new(self.parse_unary()?),
                })
            }
            _ => self.parse_primary(),
        }
    }

    fn parse_primary(&mut self) -> Result<Expr, String> {
        match self.bump() {
            Tok::Number(n) => Ok(Expr::Number(n)),
            Tok::String(s) => Ok(Expr::String(s)),
            Tok::KwTrue => Ok(Expr::Bool(true)),
            Tok::KwFalse => Ok(Expr::Bool(false)),
            Tok::KwNull => Ok(Expr::Null),
            Tok::KwIf => self.parse_if_expr(),
            Tok::KwWhile => self.parse_while_expr(),
            Tok::KwLoop => {
                // `loop while cond { ... }`
                self.eat(&Tok::KwWhile)?;
                self.parse_while_expr()
            }
            Tok::Ident(name) => {
                if matches!(self.peek(), Tok::LParen) {
                    self.bump();
                    let mut args = Vec::new();
                    if !matches!(self.peek(), Tok::RParen) {
                        loop {
                            args.push(self.parse_expr()?);
                            if matches!(self.peek(), Tok::Comma) {
                                self.bump();
                                continue;
                            }
                            break;
                        }
                    }
                    self.eat(&Tok::RParen)?;
                    Ok(Expr::Call { name, args })
                } else {
                    Ok(Expr::Name(name))
                }
            }
            Tok::LParen => {
                let e = self.parse_expr()?;
                self.eat(&Tok::RParen)?;
                Ok(e)
            }
            Tok::LBrace => {
                let mut body = Vec::new();
                while !matches!(self.peek(), Tok::RBrace | Tok::Eof) {
                    body.push(self.parse_stmt()?);
                }
                self.eat(&Tok::RBrace)?;
                Ok(Expr::Block(body))
            }
            other => Err(format!("unexpected_token:{other:?}")),
        }
    }

    fn parse_if_expr(&mut self) -> Result<Expr, String> {
        let cond = self.parse_expr()?;
        let then_body = self.parse_block_stmts()?;
        let else_body = if matches!(self.peek(), Tok::KwElse) {
            self.bump();
            Some(self.parse_block_stmts()?)
        } else {
            None
        };
        Ok(Expr::If {
            cond: Box::new(cond),
            then_body,
            else_body,
        })
    }

    fn parse_while_expr(&mut self) -> Result<Expr, String> {
        let cond = self.parse_expr()?;
        let body = self.parse_block_stmts()?;
        Ok(Expr::While {
            cond: Box::new(cond),
            body,
        })
    }
}

fn lex(source: &str) -> Result<Vec<Tok>, String> {
    let mut out = Vec::new();
    let chars: Vec<char> = source.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        if c == '/' && i + 1 < chars.len() && chars[i + 1] == '/' {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }
        if c == '"' {
            i += 1;
            let mut s = String::new();
            while i < chars.len() && chars[i] != '"' {
                if chars[i] == '\\' && i + 1 < chars.len() {
                    i += 1;
                    s.push(chars[i]);
                    i += 1;
                } else {
                    s.push(chars[i]);
                    i += 1;
                }
            }
            if i >= chars.len() {
                return Err("unclosed_string".into());
            }
            i += 1;
            out.push(Tok::String(s));
            continue;
        }
        if c.is_ascii_digit() {
            let start = i;
            i += 1;
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                i += 1;
            }
            let text: String = chars[start..i].iter().collect();
            let n: f64 = text
                .parse()
                .map_err(|_| format!("invalid_number:{text}"))?;
            out.push(Tok::Number(n));
            continue;
        }
        if c.is_ascii_alphabetic() || c == '_' {
            let start = i;
            i += 1;
            while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let text: String = chars[start..i].iter().collect();
            out.push(keyword_or_ident(&text));
            continue;
        }
        if i + 1 < chars.len() {
            let two: String = chars[i..i + 2].iter().collect();
            let tok = match two.as_str() {
                "==" => Some(Tok::EqEq),
                "!=" => Some(Tok::NotEq),
                "<=" => Some(Tok::Le),
                ">=" => Some(Tok::Ge),
                "&&" => Some(Tok::KwAnd),
                "||" => Some(Tok::KwOr),
                _ => None,
            };
            if let Some(t) = tok {
                out.push(t);
                i += 2;
                continue;
            }
        }
        let tok = match c {
            '+' => Tok::Plus,
            '-' => Tok::Minus,
            '*' => Tok::Star,
            '/' => Tok::Slash,
            '<' => Tok::Lt,
            '>' => Tok::Gt,
            '=' => Tok::Assign,
            '(' => Tok::LParen,
            ')' => Tok::RParen,
            '{' => Tok::LBrace,
            '}' => Tok::RBrace,
            ',' => Tok::Comma,
            '!' => Tok::Bang,
            other => return Err(format!("illegal_char:{other}")),
        };
        out.push(tok);
        i += 1;
    }
    out.push(Tok::Eof);
    Ok(out)
}

fn keyword_or_ident(text: &str) -> Tok {
    match text {
        "micro" | "fn" => Tok::KwMicro,
        "let" => Tok::KwLet,
        "return" => Tok::KwReturn,
        "if" => Tok::KwIf,
        "else" => Tok::KwElse,
        "while" => Tok::KwWhile,
        "loop" => Tok::KwLoop,
        "true" => Tok::KwTrue,
        "false" => Tok::KwFalse,
        "null" | "nil" => Tok::KwNull,
        "and" => Tok::KwAnd,
        "or" => Tok::KwOr,
        _ => Tok::Ident(text.to_string()),
    }
}
