#[macro_use]
pub(crate) mod token;

use crate::Ident;
use std::{collections::HashMap, fmt::Debug};
use token::{Token, TokenKind, TokenStream};

type ParseResult<T> = Result<T, String>;

#[derive(Debug)]
pub struct Parser<'src> {
    token_stream: TokenStream<'src>,
    next: Option<Token<'src>>,
    peek: Option<Token<'src>>,
}

impl<'src> Parser<'src> {
    pub fn new(source: &'src str) -> Self {
        let mut token_stream = TokenStream::new(source);
        let (next, peek) = (token_stream.next(), token_stream.next());

        Self {
            token_stream,
            peek,
            next,
        }
    }

    pub fn parse(mut self) -> ParseResult<Vec<Item>> {
        let mut items = Vec::with_capacity(10);
        while self.current().is_ok() {
            items.push(self.item()?);
        }

        Ok(items)
    }

    pub fn next(&mut self) -> ParseResult<Token<'src>> {
        let next = self.next;
        self.next = self.peek;
        self.peek = self.token_stream.next();

        next.ok_or_else(|| "Unexpected EOF".to_owned())
    }

    pub fn current(&mut self) -> ParseResult<Token<'src>> {
        self.next.ok_or_else(|| "Unexpected EOF".to_owned())
    }

    pub fn peek(&self) -> ParseResult<Token<'src>> {
        self.peek.ok_or_else(|| "Unexpected EOF".to_owned())
    }

    pub fn expect<T>(&mut self, tokens: T) -> ParseResult<Token<'src>>
    where
        T: Sliceable<TokenKind> + Debug + Copy,
    {
        if self.at(tokens) {
            Ok(self.next()?)
        } else {
            Err(format!("Expected {:?}, got {:?}", tokens, self.current()?))
        }
    }

    pub fn at<T>(&mut self, tokens: T) -> bool
    where
        T: Sliceable<TokenKind>,
    {
        self.current()
            .map(|tok| tokens.contains(tok.kind()))
            .unwrap_or_default()
    }
}

// Utils
impl Parser<'_> {
    fn ident(&mut self) -> ParseResult<Ident> {
        self.expect(T![Ident])
            .map(|ident| self.intern(ident.source()))
    }

    fn intern(&self, src: &str) -> Ident {
        Ident(crate::INTERNER.write().unwrap().get_or_intern(src))
    }
}

// Items
impl Parser<'_> {
    fn item(&mut self) -> ParseResult<Item> {
        Ok(match self.current()?.kind() {
            T![fn] => Item::Func(self.function_def()?),
            // T![use] => Item::Use(self.item_usage()),
            // T![module] => Item::Module(self.module_decl()),
            err => return Err(format!("Expected one of {:?}, got {:?}", T![Items], err)),
        })
    }

    fn item_usage(&mut self) -> ParseResult<Usage> {
        self.expect(T![use])?;
        let path = self.path()?;

        Ok(Usage { path })
    }

    fn module_decl(&mut self) -> ParseResult<Module> {
        self.expect(T![module])?;
        let _name = self.ident()?;
        self.expect(T![=])?;

        todo!("Indentation to distinguish between modules")
    }

    fn function_def(&mut self) -> ParseResult<FunctionDef> {
        self.expect(T![fn])?;
        let name = self.ident()?;
        let args = self.function_args()?;
        let return_type = if self.at(T![->]) {
            self.expect(T![->])?;
            Some(self._type()?)
        } else {
            None
        };
        self.expect(T![=])?;

        let mut body = Vec::with_capacity(5);
        while self.current().is_ok() && !self.at(T![Items]) {
            body.push(self.expression()?);
        }

        Ok(FunctionDef {
            name,
            args,
            return_type,
            body,
        })
    }

    fn function_args(&mut self) -> ParseResult<HashMap<Pattern, (usize, Type)>> {
        let (mut args, mut idx): (_, usize) = (HashMap::with_capacity(5), 0);

        while !self.at(T![=]) {
            let binding = self.pattern()?;
            self.expect(T![:])?;
            let ty = self._type()?;

            // TODO: Emit an error here
            args.insert(binding, (idx, ty))
                .expect("duplicate func args");
            idx += 1;
        }

        Ok(args)
    }

    fn _type(&mut self) -> ParseResult<Type> {
        Ok(match self.current()?.kind() {
            T![Ident] => Type::Path(self.path()?),
            _ => todo!(),
        })
    }

    fn pattern(&mut self) -> ParseResult<Pattern> {
        if self.at(T![Literal]) {
            Ok(Pattern::Literal(self.literal()?))
        } else if self.at(T![Ident]) {
            match self.path_or_ident()? {
                PathOrIdent::Path(path) => Ok(Pattern::Path(path)),
                PathOrIdent::Ident(ident) => Ok(Pattern::Ident(ident)),
            }
        } else {
            // TODO: Good error
            todo!("invalid pattern")
        }
    }

    fn literal(&mut self) -> ParseResult<Literal> {
        if self.at(T![Int]) {
            Ok(Literal::Int(
                self.expect(T![Int])?.source().parse().expect("invalid int"),
            ))
        } else if self.at(T![String]) {
            let source = self.expect(T![String])?.source();
            Ok(Literal::String(self.intern(&source[1..source.len() - 2])))
        } else if self.at(T![Bool]) {
            let boolean = match self.expect(T![Bool])?.kind() {
                T![True] => true,
                T![False] => false,
                _ => unreachable!(),
            };
            Ok(Literal::Bool(boolean))
        } else {
            // TODO: Good error
            panic!("Invalid literal");
        }
    }

    fn path_or_ident(&mut self) -> ParseResult<PathOrIdent> {
        let first = self.ident()?;

        if self.at(T![.]) {
            self.path_inner(first).map(PathOrIdent::Path)
        } else {
            Ok(PathOrIdent::Ident(first))
        }
    }

    fn path(&mut self) -> ParseResult<Path> {
        let start = self.ident()?;
        self.path_inner(start)
    }

    fn path_inner(&mut self, first: Ident) -> ParseResult<Path> {
        let mut segments = Vec::with_capacity(3);
        segments.push(first);

        while self.at(T![.]) {
            self.expect(T![.])?;
            segments.push(self.ident()?);
        }

        Ok(Path::new(segments))
    }

    // TODO: Pratt bullshit
    fn expression(&mut self) -> ParseResult<Expr> {
        Ok(match dbg!(self.current()?.kind()) {
            T![let] => Expr::Let(Box::new(self.let_binding()?)),
            T![ensure] => Expr::Ensure(Box::new(self.ensure_contract()?)),
            // T![match] => Expr::Match(self.match_expr()?),
            // T![return] => Expr::Return(self.return_expr()?),
            T![True] | T![False] | T![String] | T![Int] => Expr::Literal(self.literal()?),
            T![Ident] => Expr::Var(self.ident()?),
            // T![Ident] => Expr::Application(self.application()?),
            // T![not] => Expr::Not(self.not_expr()?),
            // T!['('] => Expr::Parens(self.parens()?),
            _ => todo!(),
        })
    }

    // TODO: Pratt
    fn precedence(_token: TokenKind) -> usize {
        todo!()
    }

    fn let_binding(&mut self) -> ParseResult<Let> {
        self.expect(T![let])?;
        let binding = self.pattern()?;
        self.expect(T![:=])?;
        let value = self.expression()?;

        Ok(Let { binding, value })
    }

    fn ensure_contract(&mut self) -> ParseResult<Ensure> {
        self.expect(T![ensure])?;
        let contract = self.expression()?;

        Ok(Ensure { contract })
    }

    fn application(&mut self) -> ParseResult<Application> {
        let function_name = self.ident()?;
        let mut arguments = Vec::with_capacity(5);

        while self.current().is_ok() {
            arguments.push(self.expression()?);
        }

        Ok(Application {
            function_name,
            arguments,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Item {
    Func(FunctionDef),
    Module(Module),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionDef {
    pub name: Ident,
    pub args: HashMap<Pattern, (usize, Type)>,
    pub return_type: Option<Type>,
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Expr {
    Let(Box<Let>),
    Ensure(Box<Ensure>),
    // Match(Match),
    // Return(Return),
    Literal(Literal),
    Var(Ident),
    Application(Application),
    // Not(Not),
    // Parens(Parens),
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
pub struct Application {
    pub function_name: Ident,
    pub arguments: Vec<Expr>,
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

    pub fn one(segment: Ident) -> Self {
        Self {
            segments: vec![segment],
        }
    }
}

pub trait Sliceable<T> {
    fn contains(&self, elem: T) -> bool;
}

impl<T: PartialEq> Sliceable<T> for T {
    fn contains(&self, elem: T) -> bool {
        self == &elem
    }
}

impl<T: PartialEq> Sliceable<T> for &[T] {
    fn contains(&self, elem: T) -> bool {
        <[T]>::contains(self, &elem)
    }
}

impl<T: PartialEq, const N: usize> Sliceable<T> for [T; N] {
    fn contains(&self, elem: T) -> bool {
        <[T]>::contains(self, &elem)
    }
}
