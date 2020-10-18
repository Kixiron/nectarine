pub use crate::Ident;

use std::fmt::{Debug, Formatter, Result as FmtResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Item {
    Func(FuncDef),
    Module(Module),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FuncDef {
    pub name: Ident,
    pub params: Vec<(Pattern, Type)>,
    pub ret: Option<Type>,
    pub body: Vec<Expr>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Module {
    pub name: Ident,
    pub items: Vec<Item>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    Path(Path),
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum Expr {
    Let(Box<Let>),
    Ensure(Box<Ensure>),
    // Match(Match),
    // Return(Return),
    Literal(Literal),
    Var(Ident),
    App(Box<App>),
    // Not(Not),
    Paren(Box<Expr>),
}

impl Debug for Expr {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Let(let_binding) => Debug::fmt(let_binding, f),
            Self::Ensure(ensure) => Debug::fmt(ensure, f),
            Self::Literal(literal) => Debug::fmt(literal, f),
            Self::Var(ident) => f.write_str(&format!("Var({:?})", ident)),
            Self::App(app) => Debug::fmt(app, f),
            Self::Paren(expr) => f.debug_tuple("Paren").field(expr).finish(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Let {
    pub binding: Pattern,
    pub value: Expr,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Pattern {
    Literal(Literal),
    Path(Path),
    Ident(Ident),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Literal {
    String(Ident),
    Int(i64),
    Bool(bool),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct App {
    pub func: Expr,
    pub arg: Expr,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Path {
    pub segments: Vec<Ident>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Usage {
    pub path: Path,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Ensure {
    pub contract: Expr,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PathOrIdent {
    Path(Path),
    Ident(Ident),
}

impl Path {
    pub const fn new(segments: Vec<Ident>) -> Self {
        Self { segments }
    }
}
