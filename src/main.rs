#![feature(once_cell, min_const_generics)]

mod ast;

use lalrpop_util::lalrpop_mod;

lalrpop_mod!(grammar);

/*
use std::{
    fmt::{Debug, Formatter, Result as FmtResult},
    lazy::SyncLazy,
    ops::Deref,
    sync::RwLock,
};
use fxhash::FxBuildHasher;
use lasso::{Rodeo, Spur};

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
*/

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct Ident(String);

use differential_datalog::{
    ddval::{DDValConvert, DDValue},
    program::{RelId, Update},
    record::{Record, UpdCmd},
    DDlog, DeltaMap,
};
use typecheck_ddlog::api::HDDlog;
use types::*;
use value::{relid2name, Relations, Value};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source = "\
        fn main =\n    \
            let x := 5;\n    \
            add <| x 5;\n\
    ";

    let items = grammar::ItemsParser::new().parse(source)?;
    println!("{:#?}", items);

    let (mut hddlog, init_state) = HDDlog::run(2, false, |_: usize, _: &Record, _: isize| {})?;

    println!("Initial state");
    dump_delta(&init_state);

    hddlog.transaction_start()?;

    let mut updates = Vec::new();
    let mut func_id = 0;
    let mut scope = 0;
    let mut expr_id = 0;

    for item in items {
        match item {
            ast::Item::Func(func) => {
                let function_id = func_id;
                func_id += 1;
                let func_scope = scope;
                scope += 1;

                updates.push(Update::Insert {
                    relid: Relations::Function as RelId,
                    v: Value::Function(Function {
                        name: internment::intern(&func.name.0),
                        id: function_id,
                        ret: func.ret.map(ddlog_type).into(),
                        scope: func_scope,
                    })
                    .into_ddvalue(),
                });

                for (pat, ty) in func.params {
                    if let ast::Pattern::Ident(name) = pat {
                        updates.push(Update::Insert {
                            relid: Relations::FuncArg as RelId,
                            v: Value::FuncArg(FuncArg {
                                func: function_id,
                                name: internment::intern(&name.0),
                                ty: ddlog_type(ty),
                            })
                            .into_ddvalue(),
                        });
                    } else {
                        todo!()
                    }
                }

                for expr in func.body {
                    let expr_scope = scope;
                    scope += 1;
                    let expression_id = expr_id;
                    expr_id += 1;

                    updates.push(Update::Insert {
                        relid: Relations::Expression as RelId,
                        v: Value::Expression(Expression {
                            id: expression_id,
                            func: function_id,
                            kind: expr_kind(&expr),
                            scope: expr_scope,
                        })
                        .into_ddvalue(),
                    });

                    match expr {
                        ast::Expr::Let(binding) => {
                            // TODO: Decl rhs
                            updates.push(Update::Insert {
                                relid: Relations::VarDecl as RelId,
                                v: Value::VarDecl(VarDecl {
                                    expr: expression_id,
                                    name: if let ast::Pattern::Ident(ident) = binding.binding {
                                        internment::intern(&ident.0)
                                    } else {
                                        todo!()
                                    },
                                    // TODO: Decl rhs
                                    val: 0,
                                })
                                .into_ddvalue(),
                            });
                        }

                        ast::Expr::Literal(lit) => {
                            updates.push(Update::Insert {
                                relid: Relations::Literal as RelId,
                                v: Value::Literal(Literal {
                                    expr: expression_id,
                                    lit: ddlog_literal(lit),
                                })
                                .into_ddvalue(),
                            });
                        }

                        ast::Expr::App(app) => {
                            // TODO: Application lhs and rhs
                            updates.push(Update::Insert {
                                relid: Relations::Application as RelId,
                                v: Value::Application(Application {
                                    expr: expression_id,
                                    // TODO: Recursively do expressions
                                    func: 0,
                                })
                                .into_ddvalue(),
                            });

                            // TODO: App args
                        }

                        ast::Expr::Var(_) => {}

                        _ => todo!(),
                    }
                }
            }

            ast::Item::Module(_) => todo!(),
        }
    }

    hddlog.apply_valupdates(updates.into_iter())?;
    let delta = hddlog.transaction_commit_dump_changes()?;

    println!("State after transaction");
    dump_delta(&delta);

    Ok(())
}

fn ddlog_literal(lit: ast::Literal) -> Lit {
    match lit {
        ast::Literal::String(s) => Lit::LitStr {
            s: internment::intern(&s.0),
        },
        ast::Literal::Int(i) => Lit::LitInt { i },
        ast::Literal::Bool(b) => Lit::LitBool { b },
    }
}

fn ddlog_type(ty: ast::Type) -> Type {
    match ty {
        ast::Type::Path(_) => Type::Unknown,
        ast::Type::Bool => Type::Bool,
        ast::Type::Int => Type::Int,
        ast::Type::String => Type::String,
    }
}

fn expr_kind(expr: &ast::Expr) -> ExprKind {
    match expr {
        ast::Expr::Let(binding) => ExprKind::Decl,
        ast::Expr::Literal(literal) => ExprKind::Lit,
        ast::Expr::Var(var) => ExprKind::Var {
            v: internment::intern(&var.0),
        },
        ast::Expr::App(app) => ExprKind::App,

        _ => todo!(),
    }
}

fn dump_delta(delta: &DeltaMap<DDValue>) {
    for (rel, changes) in delta.iter() {
        println!("Changes to relation {}", relid2name(*rel).unwrap());
        for (val, weight) in changes.iter() {
            println!(">> {} {:+}", val, weight);
        }
    }
}
