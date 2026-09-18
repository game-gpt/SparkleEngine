//! Valkyrie 子集递归下降。
//!
//! 上游 `oak-valkyrie` Builder 对表达式语句还原仍不稳定，本前端用手写子集
//! 覆盖 `micro` / `let` / `return` / `if` / `while` / 调用，保证归一到 `spark-vm`。
//!
//! 词法错误携带字节 [`SourceSpan`]，供结构化脚本错误挂接。

use std::sync::Arc;

use spark_diagnostics::SourceSpan;

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

#[derive(Debug, Clone, PartialEq)]
struct Spanned {
    kind: Tok,
    span: SourceSpan,
}

/// 解析失败：机器令牌 + 源码范围（非用户句子）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ParseFail {
    pub reason: Arc<str>,
    pub span: SourceSpan,
}

impl ParseFail {
    fn new(reason: impl Into<Arc<str>>, span: SourceSpan) -> Self {
        Self {
            reason: reason.into(),
            span,
        }
    }

    fn at(reason: impl Into<Arc<str>>, start: usize, end: usize) -> Self {
        Self::new(reason, SourceSpan::new(start, end))
    }
}

pub(crate) fn parse(source: &str) -> Result<ValkyrieRoot, ParseFail> {
    let tokens = lex(source)?;
    let mut p = Parser { tokens, i: 0 };
    p.parse_root()
}

struct Parser {
    tokens: Vec<Spanned>,
    i: usize,
}

impl Parser {
    fn peek(&self) -> &Spanned {
        self.tokens
            .get(self.i)
            .unwrap_or_else(|| self.tokens.last().expect("lex always emits Eof"))
    }

    fn peek_kind(&self) -> &Tok {
        &self.peek().kind
    }

    fn bump(&mut self) -> Spanned {
        let t = self.peek().clone();
        if self.i + 1 < self.tokens.len() {
            self.i += 1;
        }
        t
    }

    fn fail(&self, reason: impl Into<Arc<str>>) -> ParseFail {
        ParseFail::new(reason, self.peek().span)
    }

    fn eat(&mut self, expect: &Tok) -> Result<(), ParseFail> {
        let got = self.bump();
        if &got.kind == expect {
            Ok(())
        } else {
            Err(ParseFail::new(
                format!("expected_token:{expect:?}:got:{:?}", got.kind),
                got.span,
            ))
        }
    }

    fn parse_root(&mut self) -> Result<ValkyrieRoot, ParseFail> {
        let mut items = Vec::new();
        while !matches!(self.peek_kind(), Tok::Eof) {
            if matches!(self.peek_kind(), Tok::KwMicro) {
                items.push(Item::Micro(self.parse_micro()?));
            } else {
                items.push(Item::Stmt(self.parse_stmt()?));
            }
        }
        Ok(ValkyrieRoot { items })
    }

    fn parse_micro(&mut self) -> Result<Micro, ParseFail> {
        self.eat(&Tok::KwMicro)?;
        let name = match self.bump() {
            Spanned {
                kind: Tok::Ident(name),
                ..
            } => name,
            other => {
                return Err(ParseFail::new(
                    format!("expected_micro_name:{:?}", other.kind),
                    other.span,
                ));
            }
        };
        self.eat(&Tok::LParen)?;
        let mut params = Vec::new();
        if !matches!(self.peek_kind(), Tok::RParen) {
            loop {
                match self.bump() {
                    Spanned {
                        kind: Tok::Ident(p),
                        ..
                    } => params.push(p),
                    other => {
                        return Err(ParseFail::new(
                            format!("expected_param_name:{:?}", other.kind),
                            other.span,
                        ));
                    }
                }
                if matches!(self.peek_kind(), Tok::Comma) {
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

    fn parse_block_stmts(&mut self) -> Result<Vec<Stmt>, ParseFail> {
        self.eat(&Tok::LBrace)?;
        let mut body = Vec::new();
        while !matches!(self.peek_kind(), Tok::RBrace | Tok::Eof) {
            body.push(self.parse_stmt()?);
        }
        self.eat(&Tok::RBrace)?;
        Ok(body)
    }

    fn parse_stmt(&mut self) -> Result<Stmt, ParseFail> {
        match self.peek_kind() {
            Tok::KwLet => {
                self.bump();
                let name = match self.bump() {
                    Spanned {
                        kind: Tok::Ident(name),
                        ..
                    } => name,
                    other => {
                        return Err(ParseFail::new(
                            format!("expected_binding_name:{:?}", other.kind),
                            other.span,
                        ));
                    }
                };
                self.eat(&Tok::Assign)?;
                let value = self.parse_expr()?;
                Ok(Stmt::Let { name, value })
            }
            Tok::KwReturn => {
                self.bump();
                if matches!(
                    self.peek_kind(),
                    Tok::RBrace | Tok::Eof | Tok::KwElse | Tok::KwMicro
                ) {
                    Ok(Stmt::Return(None))
                } else {
                    Ok(Stmt::Return(Some(self.parse_expr()?)))
                }
            }
            _ => Ok(Stmt::Expr(self.parse_expr()?)),
        }
    }

    fn parse_expr(&mut self) -> Result<Expr, ParseFail> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Result<Expr, ParseFail> {
        let mut lhs = self.parse_and()?;
        while matches!(self.peek_kind(), Tok::KwOr) {
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

    fn parse_and(&mut self) -> Result<Expr, ParseFail> {
        let mut lhs = self.parse_cmp()?;
        while matches!(self.peek_kind(), Tok::KwAnd) {
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

    fn parse_cmp(&mut self) -> Result<Expr, ParseFail> {
        let mut lhs = self.parse_add()?;
        loop {
            let op = match self.peek_kind() {
                Tok::EqEq => BinOp::Eq,
                Tok::NotEq => BinOp::Ne,
                Tok::Lt => BinOp::Lt,
                Tok::Le => BinOp::Le,
                Tok::Gt => BinOp::Gt,
                Tok::Ge => BinOp::Ge,
                _ => break,
            };
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

    fn parse_add(&mut self) -> Result<Expr, ParseFail> {
        let mut lhs = self.parse_mul()?;
        loop {
            let op = match self.peek_kind() {
                Tok::Plus => BinOp::Add,
                Tok::Minus => BinOp::Sub,
                _ => break,
            };
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

    fn parse_mul(&mut self) -> Result<Expr, ParseFail> {
        let mut lhs = self.parse_unary()?;
        loop {
            let op = match self.peek_kind() {
                Tok::Star => BinOp::Mul,
                Tok::Slash => BinOp::Div,
                _ => break,
            };
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

    fn parse_unary(&mut self) -> Result<Expr, ParseFail> {
        match self.peek_kind() {
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

    fn parse_primary(&mut self) -> Result<Expr, ParseFail> {
        match self.peek_kind().clone() {
            Tok::KwIf => {
                self.bump();
                self.parse_if_expr()
            }
            Tok::KwWhile | Tok::KwLoop => {
                self.bump();
                self.parse_while_expr()
            }
            Tok::Number(n) => {
                self.bump();
                Ok(Expr::Number(n))
            }
            Tok::String(s) => {
                self.bump();
                Ok(Expr::String(s))
            }
            Tok::KwTrue => {
                self.bump();
                Ok(Expr::Bool(true))
            }
            Tok::KwFalse => {
                self.bump();
                Ok(Expr::Bool(false))
            }
            Tok::KwNull => {
                self.bump();
                Ok(Expr::Null)
            }
            Tok::Ident(name) => {
                self.bump();
                if matches!(self.peek_kind(), Tok::LParen) {
                    self.bump();
                    let mut args = Vec::new();
                    if !matches!(self.peek_kind(), Tok::RParen) {
                        loop {
                            args.push(self.parse_expr()?);
                            if matches!(self.peek_kind(), Tok::Comma) {
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
                self.bump();
                let e = self.parse_expr()?;
                self.eat(&Tok::RParen)?;
                Ok(e)
            }
            Tok::LBrace => {
                let mut body = Vec::new();
                self.bump();
                while !matches!(self.peek_kind(), Tok::RBrace | Tok::Eof) {
                    body.push(self.parse_stmt()?);
                }
                self.eat(&Tok::RBrace)?;
                Ok(Expr::Block(body))
            }
            other => Err(self.fail(format!("unexpected_token:{other:?}"))),
        }
    }

    fn parse_if_expr(&mut self) -> Result<Expr, ParseFail> {
        let cond = self.parse_expr()?;
        let then_body = self.parse_block_stmts()?;
        let else_body = if matches!(self.peek_kind(), Tok::KwElse) {
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

    fn parse_while_expr(&mut self) -> Result<Expr, ParseFail> {
        let cond = self.parse_expr()?;
        let body = self.parse_block_stmts()?;
        Ok(Expr::While {
            cond: Box::new(cond),
            body,
        })
    }
}

fn push_tok(out: &mut Vec<Spanned>, kind: Tok, start: usize, end: usize) {
    out.push(Spanned {
        kind,
        span: SourceSpan::new(start, end),
    });
}

fn lex(source: &str) -> Result<Vec<Spanned>, ParseFail> {
    let mut out = Vec::new();
    let bytes = source.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let start = i;
        let c = bytes[i] as char;
        if c.is_ascii_whitespace() {
            i += 1;
            continue;
        }
        if c == '/' && i + 1 < bytes.len() && bytes[i + 1] == b'/' {
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        if c == '"' {
            i += 1;
            let mut s = String::new();
            while i < bytes.len() && bytes[i] != b'"' {
                if bytes[i] == b'\\' && i + 1 < bytes.len() {
                    i += 1;
                    s.push(bytes[i] as char);
                    i += 1;
                } else {
                    s.push(bytes[i] as char);
                    i += 1;
                }
            }
            if i >= bytes.len() {
                return Err(ParseFail::at("unclosed_string", start, source.len()));
            }
            i += 1;
            push_tok(&mut out, Tok::String(s), start, i);
            continue;
        }
        if c.is_ascii_digit() {
            i += 1;
            while i < bytes.len() {
                let ch = bytes[i] as char;
                if ch.is_ascii_digit() || ch == '.' {
                    i += 1;
                } else {
                    break;
                }
            }
            let text = &source[start..i];
            let n: f64 = text
                .parse()
                .map_err(|_| ParseFail::at(format!("invalid_number:{text}"), start, i))?;
            push_tok(&mut out, Tok::Number(n), start, i);
            continue;
        }
        if c.is_ascii_alphabetic() || c == '_' {
            i += 1;
            while i < bytes.len() {
                let ch = bytes[i] as char;
                if ch.is_ascii_alphanumeric() || ch == '_' {
                    i += 1;
                } else {
                    break;
                }
            }
            let text = &source[start..i];
            push_tok(&mut out, keyword_or_ident(text), start, i);
            continue;
        }
        if i + 1 < bytes.len() {
            let two = &source[i..i + 2];
            let tok = match two {
                "==" => Some(Tok::EqEq),
                "!=" => Some(Tok::NotEq),
                "<=" => Some(Tok::Le),
                ">=" => Some(Tok::Ge),
                "&&" => Some(Tok::KwAnd),
                "||" => Some(Tok::KwOr),
                _ => None,
            };
            if let Some(t) = tok {
                push_tok(&mut out, t, start, i + 2);
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
            other => {
                return Err(ParseFail::at(format!("illegal_char:{other}"), start, start + 1));
            }
        };
        push_tok(&mut out, tok, start, start + 1);
        i += 1;
    }
    push_tok(&mut out, Tok::Eof, source.len(), source.len());
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
