#![feature(once_cell, min_const_generics)]

mod parse;

use fxhash::FxBuildHasher;
use lasso::{Rodeo, Spur};
use parse::Parser;
use std::{
    fmt::{Debug, Formatter, Result as FmtResult},
    lazy::SyncLazy,
    ops::Deref,
    sync::RwLock,
};

static INTERNER: SyncLazy<RwLock<Rodeo<Spur, FxBuildHasher>>> =
    SyncLazy::new(|| RwLock::new(Rodeo::with_hasher(FxBuildHasher::default())));

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct Ident(Spur);

impl Ident {
    fn as_str<'a>(&'a self) -> &'a str {
        // Safety: *Technically* not safe, just don't hold an outstanding reference
        //         and clear the interner or you'll have a bad time
        unsafe {
            std::mem::transmute::<&str, &'a str>(INTERNER.read().unwrap().resolve(&self.0).as_ref())
        }
    }
}

impl Deref for Ident {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl Debug for Ident {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        Debug::fmt(self.as_str(), f)
    }
}

fn main() {
    let source = "
        fn main =\n\
           let x := 5\n\
           add x 5\n\
        ";

    let parsed = Parser::new(source).parse().unwrap();
    println!("{:#?}", parsed);
}
