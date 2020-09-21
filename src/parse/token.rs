use logos::Logos;

#[derive(Logos, Debug, Copy, Clone, PartialEq, Eq)]
pub enum Token {
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
    (fn)     => { $crate::parse::token::Token::Fn };
    (use)    => { $crate::parse::token::Token::Use };
    (module) => { $crate::parse::token::Token::Module };
    (let)    => { $crate::parse::token::Token::Let };
    (match)  => { $crate::parse::token::Token::Match };
    (with)   => { $crate::parse::token::Token::With };
    (ensure) => { $crate::parse::token::Token::Ensure };
    (not)    => { $crate::parse::token::Token::Not };
    (return) => { $crate::parse::token::Token::Return };

    (Ident) => { $crate::parse::token::Token::Ident };

    (Comparison) => { [T![==], T![!=], T![>=], T![<=], T![>], T![<]] };
    (==)         => { $crate::parse::token::Token::Eq };
    (!=)         => { $crate::parse::token::Token::NotEq };
    (>=)         => { $crate::parse::token::Token::GreaterEq };
    (<=)         => { $crate::parse::token::Token::LessEq };
    (>)          => { $crate::parse::token::Token::Greater };
    (<)          => { $crate::parse::token::Token::Less };

    (Operation) => { [T![+], T![-], T![*], T![/]] };
    (+)         => { $crate::parse::token::Token::Plus };
    (-)         => { $crate::parse::token::Token::Plus };
    (*)         => { $crate::parse::token::Token::Star };
    (/)         => { $crate::parse::token::Token::FwdSlash };

    (:=) => { $crate::parse::token::Token::Assign };
    (=)  => { $crate::parse::token::Token::Equals };
    (->) => { $crate::parse::token::Token::RArrow };
    (=>) => { $crate::parse::token::Token::RRocket };

    (,)   => { $crate::parse::token::Token::Comma };
    (:)   => { $crate::parse::token::Token::Colon };
    (.)   => { $crate::parse::token::Token::Dot };
    (')') => { $crate::parse::token::Token::LParen };
    ('(') => { $crate::parse::token::Token::RParen };
    ('}') => { $crate::parse::token::Token::LBrace };
    ('{') => { $crate::parse::token::Token::RBrace };

    (String) => { $crate::parse::token::Token::String };
    (Int)    => { $crate::parse::token::Token::Int };
    (Bool)   => { [T![True], T![False]] };
    (True)   => { $crate::parse::token::Token::True };
    (False)  => { $crate::parse::token::Token::False };
}
