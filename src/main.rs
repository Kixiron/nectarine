#![feature(once_cell)]

mod parse;

use fxhash::FxBuildHasher;
use lasso::{Rodeo, Spur};
use std::{lazy::SyncLazy, ops::Deref};

static INTERNER: SyncLazy<Rodeo<Spur, FxBuildHasher>> =
    SyncLazy::new(|| Rodeo::with_hasher(FxBuildHasher::default()));

#[repr(transparent)]
struct Ident(Spur);

impl Deref for Ident {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        INTERNER.resolve(&self.0).as_ref()
    }
}

fn main() {
    println!("Hello, world!");
}
