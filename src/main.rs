#![feature(once_cell, min_const_generics)]

mod ast;

use lalrpop_util::lalrpop_mod;
use std::{cell::RefCell, marker::PhantomData, mem, rc::Rc};

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
    record::Record,
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

    let mut ddlog = Datalog::new()?;

    ddlog.transaction(move |trans| {
        let toplevel_scope = trans.scope();

        for item in items {
            match item {
                ast::Item::Func(func) => {
                    let function_id = toplevel_scope.next_func_id();
                    toplevel_scope.push_function(Function {
                        name: internment::intern(&func.name.0),
                        id: function_id,
                        ret: func.ret.map(ddlog_type).into(),
                        scope: toplevel_scope.id(),
                    });

                    let function_scope = toplevel_scope.scope();

                    for (pat, ty) in func.params {
                        if let ast::Pattern::Ident(name) = pat {
                            function_scope.push_func_arg(FuncArg {
                                func: function_id,
                                name: internment::intern(&name.0),
                                ty: ddlog_type(ty),
                            });
                        } else {
                            todo!()
                        }
                    }

                    let mut last_scope = function_scope;
                    for expr in func.body {
                        last_scope = datalog_expression(last_scope, function_id, expr).1;
                    }
                }

                ast::Item::Module(_) => todo!(),
            }
        }

        Ok(())
    })?;

    Ok(())
}

fn datalog_expression(scope: Scope<'_>, function_id: u32, expr: ast::Expr) -> (u32, Scope<'_>) {
    let expr_scope = scope.scope();
    let expression_id = expr_scope.next_expr_id();

    expr_scope.push_expression(Expression {
        id: expression_id,
        func: function_id,
        kind: expr_kind(&expr),
        scope: expr_scope.id(),
    });

    match expr {
        ast::Expr::Let(binding) => {
            // TODO: Decl rhs
            let val = datalog_expression(expr_scope.clone(), function_id, binding.value).0;
            expr_scope.push_var_decl(VarDecl {
                expr: expression_id,
                name: if let ast::Pattern::Ident(ident) = binding.binding {
                    internment::intern(&ident.0)
                } else {
                    todo!()
                },
                val,
            });
        }

        ast::Expr::Literal(lit) => {
            expr_scope.push_literal(Literal {
                expr: expression_id,
                lit: ddlog_literal(lit),
            });
        }

        ast::Expr::App(app) => {
            let func = datalog_expression(expr_scope.clone(), function_id, app.func).0;
            expr_scope.push_app(Application {
                expr: expression_id,
                func,
            });

            for arg in app.args {
                let arg = datalog_expression(expr_scope.clone(), function_id, arg).0;
                expr_scope.push_app_arg(ApplicationArg {
                    expr: expression_id,
                    arg,
                });
            }
        }

        ast::Expr::Var(_) => {}

        _ => todo!(),
    }

    (expression_id, expr_scope)
}

type DdlogResult<T> = Result<T, String>;

struct Datalog {
    datalog: Rc<RefCell<DatalogInner>>,
}

impl Datalog {
    pub fn new() -> DdlogResult<Self> {
        let (hddlog, _init_state) = HDDlog::run(2, false, |_: usize, _: &Record, _: isize| {})?;

        Ok(Self {
            datalog: Rc::new(RefCell::new(DatalogInner {
                hddlog,
                updates: Vec::with_capacity(100),
                scope_id: 0,
                function_id: 0,
                expression_id: 0,
            })),
        })
    }

    fn transaction<F>(&mut self, transaction: F) -> DdlogResult<()>
    where
        F: for<'trans> FnOnce(&mut DatalogTransaction<'trans>) -> DdlogResult<()>,
    {
        let mut trans = DatalogTransaction::new(self.datalog.clone())?;
        transaction(&mut trans)?;
        trans.commit()?;

        Ok(())
    }
}

struct DatalogInner {
    hddlog: HDDlog,
    updates: Vec<Update<DDValue>>,
    scope_id: u32,
    function_id: u32,
    expression_id: u32,
}

impl DatalogInner {
    pub fn inc_scope(&mut self) -> u32 {
        let temp = self.scope_id;
        self.scope_id += 1;
        temp
    }

    pub fn inc_function(&mut self) -> u32 {
        let temp = self.function_id;
        self.function_id += 1;
        temp
    }

    pub fn inc_expression(&mut self) -> u32 {
        let temp = self.expression_id;
        self.expression_id += 1;
        temp
    }

    fn push_scope(&mut self, scope: InputScope) {
        self.updates.push(Update::Insert {
            relid: Relations::InputScope as RelId,
            v: Value::InputScope(scope).into_ddvalue(),
        });
    }
}

struct DatalogTransaction<'ddlog> {
    datalog: Rc<RefCell<DatalogInner>>,
    __lifetime: PhantomData<&'ddlog ()>,
}

impl<'ddlog> DatalogTransaction<'ddlog> {
    fn new(datalog: Rc<RefCell<DatalogInner>>) -> DdlogResult<Self> {
        datalog.borrow_mut().hddlog.transaction_start()?;

        Ok(Self {
            datalog,
            __lifetime: PhantomData,
        })
    }

    pub fn scope(&self) -> Scope<'_> {
        let mut datalog = self.datalog.borrow_mut();
        let id = datalog.inc_scope();
        datalog.push_scope(InputScope {
            // FIXME: ???
            parent: 0,
            child: id,
        });

        Scope {
            datalog: self.datalog.clone(),
            id,
            __lifetime: PhantomData,
        }
    }

    pub fn commit(self) -> DdlogResult<()> {
        let mut datalog = self.datalog.borrow_mut();

        let updates = mem::take(&mut datalog.updates);
        datalog.hddlog.apply_valupdates(updates.into_iter())?;

        let delta = datalog.hddlog.transaction_commit_dump_changes()?;

        println!("State after transaction");
        dump_delta(&delta);

        Ok(())
    }
}

#[derive(Clone)]
struct Scope<'ddlog> {
    datalog: Rc<RefCell<DatalogInner>>,
    id: u32,
    __lifetime: PhantomData<&'ddlog ()>,
}

impl<'ddlog> Scope<'ddlog> {
    pub fn scope(&self) -> Scope<'ddlog> {
        let mut datalog = self.datalog.borrow_mut();
        let id = datalog.inc_scope();
        datalog.push_scope(InputScope {
            // FIXME: ???
            parent: self.id,
            child: id,
        });

        Scope {
            datalog: self.datalog.clone(),
            id,
            __lifetime: PhantomData,
        }
    }

    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn next_func_id(&self) -> u32 {
        self.datalog.borrow_mut().inc_function()
    }

    pub fn next_expr_id(&self) -> u32 {
        self.datalog.borrow_mut().inc_expression()
    }

    pub fn push_function(&self, func: Function) {
        self.datalog.borrow_mut().updates.push(Update::Insert {
            relid: Relations::Function as RelId,
            v: Value::Function(func).into_ddvalue(),
        });
    }

    pub fn push_func_arg(&self, arg: FuncArg) {
        self.datalog.borrow_mut().updates.push(Update::Insert {
            relid: Relations::FuncArg as RelId,
            v: Value::FuncArg(arg).into_ddvalue(),
        });
    }

    pub fn push_expression(&self, expr: Expression) {
        self.datalog.borrow_mut().updates.push(Update::Insert {
            relid: Relations::Expression as RelId,
            v: Value::Expression(expr).into_ddvalue(),
        });
    }

    pub fn push_var_decl(&self, var_decl: VarDecl) {
        self.datalog.borrow_mut().updates.push(Update::Insert {
            relid: Relations::VarDecl as RelId,
            v: Value::VarDecl(var_decl).into_ddvalue(),
        });
    }

    pub fn push_literal(&self, literal: Literal) {
        self.datalog.borrow_mut().updates.push(Update::Insert {
            relid: Relations::Literal as RelId,
            v: Value::Literal(literal).into_ddvalue(),
        });
    }

    pub fn push_app(&self, app: Application) {
        self.datalog.borrow_mut().updates.push(Update::Insert {
            relid: Relations::Application as RelId,
            v: Value::Application(app).into_ddvalue(),
        });
    }

    pub fn push_app_arg(&self, arg: ApplicationArg) {
        self.datalog.borrow_mut().updates.push(Update::Insert {
            relid: Relations::ApplicationArg as RelId,
            v: Value::ApplicationArg(arg).into_ddvalue(),
        });
    }
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
        ast::Expr::Let(_) => ExprKind::Decl,
        ast::Expr::Literal(_) => ExprKind::Lit,
        ast::Expr::Var(var) => ExprKind::Var {
            v: internment::intern(&var.0),
        },
        ast::Expr::App(_) => ExprKind::App,

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
