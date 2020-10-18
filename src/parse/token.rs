use logos::{Lexer, Logos};
use std::{convert::TryInto, fmt, ops::Range};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Token<'src> {
    kind: TokenKind,
    source: &'src str,
    span: Span,
}

impl<'src> Token<'src> {
    const fn new(kind: TokenKind, source: &'src str, span: Span) -> Self {
        Self { kind, source, span }
    }

    pub const fn kind(&self) -> TokenKind {
        self.kind
    }

    pub const fn source(&self) -> &'src str {
        self.source
    }
}

#[derive(Logos, Debug, Copy, Clone, PartialEq, Eq)]
pub enum TokenKind {
    #[token("fn")]
    Fn,
    #[token("use")]
    Use,
    #[token("module")]
    Module,
    #[token("let")]
    Let,
    #[token("match")]
    Match,
    #[token("with")]
    With,
    #[token("ensure")]
    Ensure,
    #[token("not")]
    Not,
    #[token("return")]
    Return,

    #[regex("[a-zA-Z_][a-zA-Z0-9_]*")]
    Ident,
    #[regex("\"[^\"]*\"")]
    String,
    #[regex("[0-9][0-9_]*")]
    Int,
    #[token("True")]
    True,
    #[token("False")]
    False,

    #[token("==")]
    Eq,
    #[token("!=")]
    NotEq,
    #[token(">=")]
    GreaterEq,
    #[token("<=")]
    LessEq,
    #[token(">")]
    Greater,
    #[token("<")]
    Less,

    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Star,
    #[token("/")]
    FwdSlash,

    #[token(":=")]
    Assign,
    #[token("=")]
    Equals,
    #[token("->")]
    RArrow,
    #[token("=>")]
    RRocket,

    #[token(",")]
    Comma,
    #[token(":")]
    Colon,
    #[token(".")]
    Dot,
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,

    #[error]
    // Skip comments
    #[regex("--[^\n]\n", logos::skip)]
    // Skip whitespace
    #[regex(r"[ \t\n\f]+", logos::skip)]
    Error,
}

/// Allows easily creating tokens without having to remember their names
#[rustfmt::skip]
#[doc(hidden)]
#[macro_export]
macro_rules! T {
    (Items)  => { [T![fn], T![use], T![module]] };
    (fn)     => { $crate::parse::token::TokenKind::Fn };
    (use)    => { $crate::parse::token::TokenKind::Use };
    (module) => { $crate::parse::token::TokenKind::Module };
    (let)    => { $crate::parse::token::TokenKind::Let };
    (match)  => { $crate::parse::token::TokenKind::Match };
    (with)   => { $crate::parse::token::TokenKind::With };
    (ensure) => { $crate::parse::token::TokenKind::Ensure };
    (not)    => { $crate::parse::token::TokenKind::Not };
    (return) => { $crate::parse::token::TokenKind::Return };

    (Ident) => { $crate::parse::token::TokenKind::Ident };

    (Comparison) => { [T![==], T![!=], T![>=], T![<=], T![>], T![<]] };
    (==)         => { $crate::parse::token::TokenKind::Eq };
    (!=)         => { $crate::parse::token::TokenKind::NotEq };
    (>=)         => { $crate::parse::token::TokenKind::GreaterEq };
    (<=)         => { $crate::parse::token::TokenKind::LessEq };
    (>)          => { $crate::parse::token::TokenKind::Greater };
    (<)          => { $crate::parse::token::TokenKind::Less };

    (Operation) => { [T![+], T![-], T![*], T![/]] };
    (+)         => { $crate::parse::token::TokenKind::Plus };
    (-)         => { $crate::parse::token::TokenKind::Plus };
    (*)         => { $crate::parse::token::TokenKind::Star };
    (/)         => { $crate::parse::token::TokenKind::FwdSlash };

    (:=) => { $crate::parse::token::TokenKind::Assign };
    (=)  => { $crate::parse::token::TokenKind::Equals };
    (->) => { $crate::parse::token::TokenKind::RArrow };
    (=>) => { $crate::parse::token::TokenKind::RRocket };

    (,)   => { $crate::parse::token::TokenKind::Comma };
    (:)   => { $crate::parse::token::TokenKind::Colon };
    (.)   => { $crate::parse::token::TokenKind::Dot };
    (')') => { $crate::parse::token::TokenKind::LParen };
    ('(') => { $crate::parse::token::TokenKind::RParen };
    ('}') => { $crate::parse::token::TokenKind::LBrace };
    ('{') => { $crate::parse::token::TokenKind::RBrace };

    (Literal) => { [T![String], T![Int], T![True], T![False]] };
    (String)  => { $crate::parse::token::TokenKind::String };
    (Int)     => { $crate::parse::token::TokenKind::Int };
    (Bool)    => { [T![True], T![False]] };
    (True)    => { $crate::parse::token::TokenKind::True };
    (False)   => { $crate::parse::token::TokenKind::False };
}

#[derive(Clone)]
pub struct TokenStream<'src> {
    lexer: Lexer<'src, TokenKind>,
}

impl<'src> TokenStream<'src> {
    pub fn new(input: &'src str) -> Self {
        Self {
            lexer: TokenKind::lexer(input),
        }
    }
}

impl<'src> Iterator for TokenStream<'src> {
    type Item = Token<'src>;

    fn next(&mut self) -> Option<Self::Item> {
        self.lexer
            .next()
            .map(|token| Token::new(token, self.lexer.slice(), self.lexer.span().into()))
    }
}

impl fmt::Debug for TokenStream<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let tokens = self.clone().collect::<Vec<Token<'_>>>();

        f.debug_struct("TokenStream")
            .field("lexer", &tokens)
            .finish()
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Span {
    start: u32,
    end: u32,
}

impl From<Range<u32>> for Span {
    fn from(range: Range<u32>) -> Self {
        Self {
            start: range.start,
            end: range.end,
        }
    }
}

impl From<Range<usize>> for Span {
    fn from(range: Range<usize>) -> Self {
        Self {
            start: range.start.try_into().expect("More than 4gb span"),
            end: range.end.try_into().expect("More than 4gb span"),
        }
    }
}
