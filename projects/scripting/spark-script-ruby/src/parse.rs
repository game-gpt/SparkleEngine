//! RPG Maker / RGSS 口径的手写递归下降（`oak-ruby` Builder 尚为空壳，故自研子集）。

use crate::ast::{BinOp, Expr, Item, Method, RubyRoot, Stmt, UnaryOp};

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Ident(String),
    Number(f64),
    String(String),
    KwDef,
    KwEnd,
    KwIf,
    KwElse,
    KwElsif,
    KwWhile,
    KwReturn,
    KwTrue,
    KwFalse,
    KwNil,
    KwAnd,
    KwOr,
    KwNot,
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
    Comma,
    Bang,
    Eof,
}

pub fn parse(source: &str) -> Result<RubyRoot, String> {
    let tokens = lex(source)?;
    let mut p = Parser {
        tokens,
        i: 0,
    };
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

    fn parse_root(&mut self) -> Result<RubyRoot, String> {
        let mut items = Vec::new();
        while !matches!(self.peek(), Tok::Eof) {
            if matches!(self.peek(), Tok::KwDef) {
                items.push(Item::Method(self.parse_method()?));
            } else {
                items.push(Item::Stmt(self.parse_stmt()?));
            }
        }
        Ok(RubyRoot { items })
    }

    fn parse_method(&mut self) -> Result<Method, String> {
        self.eat(&Tok::KwDef)?;
        let name = match self.bump() {
            Tok::Ident(n) => n,
            other => return Err(format!("expected_method_name:{other:?}")),
        };
        let mut params = Vec::new();
        if matches!(self.peek(), Tok::LParen) {
            self.bump();
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
        }
        let body = self.parse_body_until_end()?;
        Ok(Method { name, params, body })
    }

    fn parse_body_until_end(&mut self) -> Result<Vec<Stmt>, String> {
        let mut body = Vec::new();
        while !matches!(self.peek(), Tok::KwEnd | Tok::KwElse | Tok::KwElsif | Tok::Eof) {
            body.push(self.parse_stmt()?);
        }
        self.eat(&Tok::KwEnd)?;
        Ok(body)
    }

    fn parse_stmt(&mut self) -> Result<Stmt, String> {
        match self.peek() {
            Tok::KwReturn => {
                self.bump();
                if matches!(
                    self.peek(),
                    Tok::KwEnd | Tok::KwElse | Tok::KwElsif | Tok::Eof | Tok::KwDef
                ) {
                    Ok(Stmt::Return(None))
                } else {
                    Ok(Stmt::Return(Some(self.parse_expr()?)))
                }
            }
            Tok::KwIf => self.parse_if(),
            Tok::KwWhile => self.parse_while(),
            Tok::Ident(_) => {
                // 可能是赋值 `x = ...` 或表达式
                let save = self.i;
                if let Tok::Ident(name) = self.bump() {
                    if matches!(self.peek(), Tok::Assign) {
                        self.bump();
                        let value = self.parse_expr()?;
                        return Ok(Stmt::Assign { name, value });
                    }
                }
                self.i = save;
                Ok(Stmt::Expr(self.parse_expr()?))
            }
            _ => Ok(Stmt::Expr(self.parse_expr()?)),
        }
    }

    fn parse_if(&mut self) -> Result<Stmt, String> {
        self.eat(&Tok::KwIf)?;
        let cond = self.parse_expr()?;
        let mut then_body = Vec::new();
        while !matches!(
            self.peek(),
            Tok::KwEnd | Tok::KwElse | Tok::KwElsif | Tok::Eof
        ) {
            then_body.push(self.parse_stmt()?);
        }
        let else_body = if matches!(self.peek(), Tok::KwElse) {
            self.bump();
            let mut eb = Vec::new();
            while !matches!(self.peek(), Tok::KwEnd | Tok::Eof) {
                eb.push(self.parse_stmt()?);
            }
            Some(eb)
        } else if matches!(self.peek(), Tok::KwElsif) {
            return Err("unsupported_elsif".into());
        } else {
            None
        };
        self.eat(&Tok::KwEnd)?;
        Ok(Stmt::If {
            cond,
            then_body,
            else_body,
        })
    }

    fn parse_while(&mut self) -> Result<Stmt, String> {
        self.eat(&Tok::KwWhile)?;
        let cond = self.parse_expr()?;
        let body = self.parse_body_until_end()?;
        Ok(Stmt::While { cond, body })
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
            Tok::Bang | Tok::KwNot => {
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
            Tok::KwNil => Ok(Expr::Null),
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
            other => Err(format!("unexpected_token:{other:?}")),
        }
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
        if c == '#' {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }
        if c == '"' || c == '\'' {
            let quote = c;
            i += 1;
            let mut s = String::new();
            while i < chars.len() && chars[i] != quote {
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
        if c.is_ascii_alphabetic() || c == '_' || c == '@' {
            let start = i;
            i += 1;
            while i < chars.len()
                && (chars[i].is_ascii_alphanumeric() || chars[i] == '_' || chars[i] == '?' || chars[i] == '!')
            {
                i += 1;
            }
            let text: String = chars[start..i].iter().collect();
            out.push(keyword_or_ident(&text));
            continue;
        }
        // 多字符算符
        if i + 1 < chars.len() {
            let two: String = chars[i..i + 2].iter().collect();
            let tok = match two.as_str() {
                "==" => Some(Tok::EqEq),
                "!=" | "<>" => Some(Tok::NotEq),
                "<=" => Some(Tok::Le),
                ">=" => Some(Tok::Ge),
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
        "def" => Tok::KwDef,
        "end" => Tok::KwEnd,
        "if" => Tok::KwIf,
        "else" => Tok::KwElse,
        "elsif" => Tok::KwElsif,
        "while" => Tok::KwWhile,
        "return" => Tok::KwReturn,
        "true" => Tok::KwTrue,
        "false" => Tok::KwFalse,
        "nil" | "null" => Tok::KwNil,
        "and" => Tok::KwAnd,
        "or" => Tok::KwOr,
        "not" => Tok::KwNot,
        _ => Tok::Ident(text.to_string()),
    }
}
