#[macro_use]
pub(crate) mod token;

use crate::Ident;
use logos::Lexer;
use token::Token;

type ParseResult<T> = Result<T, String>;

pub struct Parser<'src> {
    lexer: Lexer<'src, Token>,
    next: Option<Token>,
    peek: Option<Token>,
}

impl<'src> Parser<'src> {
    pub fn new(source: &'src str) -> Self {
        let mut lexer = Lexer::new(source);
        let (next, peek) = (lexer.next(), lexer.next());

        Self { lexer, peek, next }
    }

    pub fn next(&mut self) -> ParseResult<Token> {
        let next = self.next;
        self.next = self.peek;
        self.peek = self.lexer.next();

        next.ok_or_else(|| "Unexpected EOF".to_owned())
    }

    pub fn is(&mut self, token: Token) -> bool {
        self.peek == Some(token)
    }

    pub fn peek(&self) -> ParseResult<Token> {
        self.peek.ok_or_else(|| "Unexpected EOF".to_owned())
    }

    pub fn expect(&mut self, token: Token) -> ParseResult<Token> {
        let next = self.next()?;
        if next == token {
            Ok(next)
        } else {
            Err(format!("Expected {:?}, got {:?}", token, next))
        }
    }
}

// Utils
impl Parser<'_> {
    fn ident(&mut self) -> ParseResult<Ident> {
        self.expect(T![Ident])
            .map(|_| Ident(crate::INTERNER.get_or_intern(ident)))
    }
}

// Items
impl Parser<'_> {
    fn item(&mut self) -> ParseResult<Item> {
        match self.peek()? {
            T![fn] => self.function_def().map(Into::into),
            T![use] => self.usage().map(Into::into),
            T![module] => self.module().map(Into::into),

            err => Err(format!("Expected one of {:?}, got {:?}", T![Items], err)),
        }
    }

    fn function_def(&mut self) -> ParseResult<FunctionDef> {
        self.expect(T![fn])?;
        let name = self.ident()?;
        let args = self.function_args()?;
        let return_type = if self.is(T![->]) {
            self.expect(T![->])?;
            Some(self._type()?)
        } else {
            None
        };
        self.expect(T![=])?;

        let mut body = Vec::with_capacity(5);
        while !T![Items].contains(&self.peek()?) {
            body.push(self.expression()?);
        }

        Ok(FunctionDef {
            name,
            args,
            return_type,
            body,
        })
    }
}
