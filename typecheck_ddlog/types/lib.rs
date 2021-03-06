#![allow(
    path_statements,
    unused_imports,
    non_snake_case,
    non_camel_case_types,
    non_upper_case_globals,
    unused_parens,
    non_shorthand_field_patterns,
    dead_code,
    overflowing_literals,
    unreachable_patterns,
    unused_variables,
    clippy::unknown_clippy_lints,
    clippy::missing_safety_doc,
    clippy::match_single_binding
)]

//use ::serde::de::DeserializeOwned;
use ::differential_datalog::record::FromRecord;
use ::differential_datalog::record::IntoRecord;
use ::differential_datalog::record::Mutator;
use ::serde::Deserialize;
use ::serde::Serialize;

// `usize` and `isize` are builtin Rust types; we therefore declare an alias to DDlog's `usize` and
// `isize`.
pub type std_usize = u64;
pub type std_isize = i64;

mod ddlog_log;
pub use ddlog_log::*;

pub mod closure;

/* FlatBuffers code generated by `flatc` */
#[cfg(feature = "flatbuf")]
mod flatbuf_generated;

/* `FromFlatBuffer`, `ToFlatBuffer`, etc, trait declarations. */
#[cfg(feature = "flatbuf")]
pub mod flatbuf;

pub trait Val:
    Default
    + Eq
    + Ord
    + Clone
    + ::std::hash::Hash
    + PartialEq
    + PartialOrd
    + Serialize
    + ::serde::de::DeserializeOwned
    + 'static
{
}

impl<T> Val for T where
    T: Default
        + Eq
        + Ord
        + Clone
        + ::std::hash::Hash
        + PartialEq
        + PartialOrd
        + Serialize
        + ::serde::de::DeserializeOwned
        + 'static
{
}

pub fn string_append_str(mut s1: String, s2: &str) -> String {
    s1.push_str(s2);
    s1
}

#[allow(clippy::ptr_arg)]
pub fn string_append(mut s1: String, s2: &String) -> String {
    s1.push_str(s2.as_str());
    s1
}

#[macro_export]
macro_rules! deserialize_map_from_array {
    ( $modname:ident, $ktype:ty, $vtype:ty, $kfunc:path ) => {
        mod $modname {
            use super::*;
            use serde::de::{Deserialize, Deserializer};
            use serde::ser::Serializer;
            use std::collections::BTreeMap;

            pub fn serialize<S>(
                map: &crate::ddlog_std::Map<$ktype, $vtype>,
                serializer: S,
            ) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                serializer.collect_seq(map.x.values())
            }

            pub fn deserialize<'de, D>(
                deserializer: D,
            ) -> Result<crate::ddlog_std::Map<$ktype, $vtype>, D::Error>
            where
                D: Deserializer<'de>,
            {
                let v = Vec::<$vtype>::deserialize(deserializer)?;
                Ok(v.into_iter().map(|item| ($kfunc(&item), item)).collect())
            }
        }
    };
}


pub mod ddlog_std;
pub mod internment;
pub mod debug;
pub mod log;
pub mod vec;
#[derive(Eq, Ord, Clone, Hash, PartialEq, PartialOrd, Default, Serialize, Deserialize)]
pub struct Application {
    pub expr: crate::ExprId,
    pub func: crate::ExprId
}
impl abomonation::Abomonation for Application{}
::differential_datalog::decl_struct_from_record!(Application["Application"]<>, ["Application"][2]{[0]expr["expr"]: crate::ExprId, [1]func["func"]: crate::ExprId});
::differential_datalog::decl_struct_into_record!(Application, ["Application"]<>, expr, func);
::differential_datalog::decl_record_mutator_struct!(Application, <>, expr: crate::ExprId, func: crate::ExprId);
impl ::std::fmt::Display for Application {
    fn fmt(&self, __formatter: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        match self {
            crate::Application{expr,func} => {
                __formatter.write_str("Application{")?;
                ::std::fmt::Debug::fmt(expr, __formatter)?;
                __formatter.write_str(",")?;
                ::std::fmt::Debug::fmt(func, __formatter)?;
                __formatter.write_str("}")
            }
        }
    }
}
impl ::std::fmt::Debug for Application {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        ::std::fmt::Display::fmt(&self, f)
    }
}
#[derive(Eq, Ord, Clone, Hash, PartialEq, PartialOrd, Default, Serialize, Deserialize)]
pub struct ApplicationArg {
    pub expr: crate::ExprId,
    pub arg: crate::ExprId
}
impl abomonation::Abomonation for ApplicationArg{}
::differential_datalog::decl_struct_from_record!(ApplicationArg["ApplicationArg"]<>, ["ApplicationArg"][2]{[0]expr["expr"]: crate::ExprId, [1]arg["arg"]: crate::ExprId});
::differential_datalog::decl_struct_into_record!(ApplicationArg, ["ApplicationArg"]<>, expr, arg);
::differential_datalog::decl_record_mutator_struct!(ApplicationArg, <>, expr: crate::ExprId, arg: crate::ExprId);
impl ::std::fmt::Display for ApplicationArg {
    fn fmt(&self, __formatter: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        match self {
            crate::ApplicationArg{expr,arg} => {
                __formatter.write_str("ApplicationArg{")?;
                ::std::fmt::Debug::fmt(expr, __formatter)?;
                __formatter.write_str(",")?;
                ::std::fmt::Debug::fmt(arg, __formatter)?;
                __formatter.write_str("}")
            }
        }
    }
}
impl ::std::fmt::Debug for ApplicationArg {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        ::std::fmt::Display::fmt(&self, f)
    }
}
#[derive(Eq, Ord, Clone, Hash, PartialEq, PartialOrd, Default, Serialize, Deserialize)]
pub struct ChildScope {
    pub parent: crate::Scope,
    pub child: crate::Scope
}
impl abomonation::Abomonation for ChildScope{}
::differential_datalog::decl_struct_from_record!(ChildScope["ChildScope"]<>, ["ChildScope"][2]{[0]parent["parent"]: crate::Scope, [1]child["child"]: crate::Scope});
::differential_datalog::decl_struct_into_record!(ChildScope, ["ChildScope"]<>, parent, child);
::differential_datalog::decl_record_mutator_struct!(ChildScope, <>, parent: crate::Scope, child: crate::Scope);
impl ::std::fmt::Display for ChildScope {
    fn fmt(&self, __formatter: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        match self {
            crate::ChildScope{parent,child} => {
                __formatter.write_str("ChildScope{")?;
                ::std::fmt::Debug::fmt(parent, __formatter)?;
                __formatter.write_str(",")?;
                ::std::fmt::Debug::fmt(child, __formatter)?;
                __formatter.write_str("}")
            }
        }
    }
}
impl ::std::fmt::Debug for ChildScope {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        ::std::fmt::Display::fmt(&self, f)
    }
}
pub type ExprId = u32;
#[derive(Eq, Ord, Clone, Hash, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum ExprKind {
    Var {
        v: crate::Ident
    },
    App,
    Decl,
    Lit
}
impl abomonation::Abomonation for ExprKind{}
::differential_datalog::decl_enum_from_record!(ExprKind["ExprKind"]<>, Var["Var"][1]{[0]v["v"]: crate::Ident}, App["App"][0]{}, Decl["Decl"][0]{}, Lit["Lit"][0]{});
::differential_datalog::decl_enum_into_record!(ExprKind<>, Var["Var"]{v}, App["App"]{}, Decl["Decl"]{}, Lit["Lit"]{});
::differential_datalog::decl_record_mutator_enum!(ExprKind<>, Var{v: crate::Ident}, App{}, Decl{}, Lit{});
impl ::std::fmt::Display for ExprKind {
    fn fmt(&self, __formatter: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        match self {
            crate::ExprKind::Var{v} => {
                __formatter.write_str("Var{")?;
                ::std::fmt::Debug::fmt(v, __formatter)?;
                __formatter.write_str("}")
            },
            crate::ExprKind::App{} => {
                __formatter.write_str("App{")?;
                __formatter.write_str("}")
            },
            crate::ExprKind::Decl{} => {
                __formatter.write_str("Decl{")?;
                __formatter.write_str("}")
            },
            crate::ExprKind::Lit{} => {
                __formatter.write_str("Lit{")?;
                __formatter.write_str("}")
            }
        }
    }
}
impl ::std::fmt::Debug for ExprKind {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        ::std::fmt::Display::fmt(&self, f)
    }
}
impl ::std::default::Default for ExprKind {
    fn default() -> Self {
        crate::ExprKind::Var{v : ::std::default::Default::default()}
    }
}
#[derive(Eq, Ord, Clone, Hash, PartialEq, PartialOrd, Default, Serialize, Deserialize)]
pub struct Expression {
    pub id: crate::ExprId,
    pub func: crate::FuncId,
    pub kind: crate::ExprKind,
    pub scope: crate::Scope
}
impl abomonation::Abomonation for Expression{}
::differential_datalog::decl_struct_from_record!(Expression["Expression"]<>, ["Expression"][4]{[0]id["id"]: crate::ExprId, [1]func["func"]: crate::FuncId, [2]kind["kind"]: crate::ExprKind, [3]scope["scope"]: crate::Scope});
::differential_datalog::decl_struct_into_record!(Expression, ["Expression"]<>, id, func, kind, scope);
::differential_datalog::decl_record_mutator_struct!(Expression, <>, id: crate::ExprId, func: crate::FuncId, kind: crate::ExprKind, scope: crate::Scope);
impl ::std::fmt::Display for Expression {
    fn fmt(&self, __formatter: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        match self {
            crate::Expression{id,func,kind,scope} => {
                __formatter.write_str("Expression{")?;
                ::std::fmt::Debug::fmt(id, __formatter)?;
                __formatter.write_str(",")?;
                ::std::fmt::Debug::fmt(func, __formatter)?;
                __formatter.write_str(",")?;
                ::std::fmt::Debug::fmt(kind, __formatter)?;
                __formatter.write_str(",")?;
                ::std::fmt::Debug::fmt(scope, __formatter)?;
                __formatter.write_str("}")
            }
        }
    }
}
impl ::std::fmt::Debug for Expression {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        ::std::fmt::Display::fmt(&self, f)
    }
}
#[derive(Eq, Ord, Clone, Hash, PartialEq, PartialOrd, Default, Serialize, Deserialize)]
pub struct ExpressionType {
    pub expr: crate::ExprId,
    pub ty: crate::Type
}
impl abomonation::Abomonation for ExpressionType{}
::differential_datalog::decl_struct_from_record!(ExpressionType["ExpressionType"]<>, ["ExpressionType"][2]{[0]expr["expr"]: crate::ExprId, [1]ty["ty"]: crate::Type});
::differential_datalog::decl_struct_into_record!(ExpressionType, ["ExpressionType"]<>, expr, ty);
::differential_datalog::decl_record_mutator_struct!(ExpressionType, <>, expr: crate::ExprId, ty: crate::Type);
impl ::std::fmt::Display for ExpressionType {
    fn fmt(&self, __formatter: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        match self {
            crate::ExpressionType{expr,ty} => {
                __formatter.write_str("ExpressionType{")?;
                ::std::fmt::Debug::fmt(expr, __formatter)?;
                __formatter.write_str(",")?;
                ::std::fmt::Debug::fmt(ty, __formatter)?;
                __formatter.write_str("}")
            }
        }
    }
}
impl ::std::fmt::Debug for ExpressionType {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        ::std::fmt::Display::fmt(&self, f)
    }
}
#[derive(Eq, Ord, Clone, Hash, PartialEq, PartialOrd, Default, Serialize, Deserialize)]
pub struct FuncArg {
    pub func: crate::FuncId,
    pub name: crate::Ident,
    pub ty: crate::Type
}
impl abomonation::Abomonation for FuncArg{}
::differential_datalog::decl_struct_from_record!(FuncArg["FuncArg"]<>, ["FuncArg"][3]{[0]func["func"]: crate::FuncId, [1]name["name"]: crate::Ident, [2]ty["ty"]: crate::Type});
::differential_datalog::decl_struct_into_record!(FuncArg, ["FuncArg"]<>, func, name, ty);
::differential_datalog::decl_record_mutator_struct!(FuncArg, <>, func: crate::FuncId, name: crate::Ident, ty: crate::Type);
impl ::std::fmt::Display for FuncArg {
    fn fmt(&self, __formatter: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        match self {
            crate::FuncArg{func,name,ty} => {
                __formatter.write_str("FuncArg{")?;
                ::std::fmt::Debug::fmt(func, __formatter)?;
                __formatter.write_str(",")?;
                ::std::fmt::Debug::fmt(name, __formatter)?;
                __formatter.write_str(",")?;
                ::std::fmt::Debug::fmt(ty, __formatter)?;
                __formatter.write_str("}")
            }
        }
    }
}
impl ::std::fmt::Debug for FuncArg {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        ::std::fmt::Display::fmt(&self, f)
    }
}
pub type FuncId = u32;
#[derive(Eq, Ord, Clone, Hash, PartialEq, PartialOrd, Default, Serialize, Deserialize)]
pub struct Function {
    pub name: crate::Ident,
    pub id: crate::FuncId,
    pub scope: crate::Scope,
    pub ret: crate::ddlog_std::Option<crate::Type>
}
impl abomonation::Abomonation for Function{}
::differential_datalog::decl_struct_from_record!(Function["Function"]<>, ["Function"][4]{[0]name["name"]: crate::Ident, [1]id["id"]: crate::FuncId, [2]scope["scope"]: crate::Scope, [3]ret["ret"]: crate::ddlog_std::Option<crate::Type>});
::differential_datalog::decl_struct_into_record!(Function, ["Function"]<>, name, id, scope, ret);
::differential_datalog::decl_record_mutator_struct!(Function, <>, name: crate::Ident, id: crate::FuncId, scope: crate::Scope, ret: crate::ddlog_std::Option<crate::Type>);
impl ::std::fmt::Display for Function {
    fn fmt(&self, __formatter: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        match self {
            crate::Function{name,id,scope,ret} => {
                __formatter.write_str("Function{")?;
                ::std::fmt::Debug::fmt(name, __formatter)?;
                __formatter.write_str(",")?;
                ::std::fmt::Debug::fmt(id, __formatter)?;
                __formatter.write_str(",")?;
                ::std::fmt::Debug::fmt(scope, __formatter)?;
                __formatter.write_str(",")?;
                ::std::fmt::Debug::fmt(ret, __formatter)?;
                __formatter.write_str("}")
            }
        }
    }
}
impl ::std::fmt::Debug for Function {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        ::std::fmt::Display::fmt(&self, f)
    }
}
pub type Ident = crate::internment::istring;
#[derive(Eq, Ord, Clone, Hash, PartialEq, PartialOrd, Default, Serialize, Deserialize)]
pub struct InputScope {
    pub parent: crate::Scope,
    pub child: crate::Scope
}
impl abomonation::Abomonation for InputScope{}
::differential_datalog::decl_struct_from_record!(InputScope["InputScope"]<>, ["InputScope"][2]{[0]parent["parent"]: crate::Scope, [1]child["child"]: crate::Scope});
::differential_datalog::decl_struct_into_record!(InputScope, ["InputScope"]<>, parent, child);
::differential_datalog::decl_record_mutator_struct!(InputScope, <>, parent: crate::Scope, child: crate::Scope);
impl ::std::fmt::Display for InputScope {
    fn fmt(&self, __formatter: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        match self {
            crate::InputScope{parent,child} => {
                __formatter.write_str("InputScope{")?;
                ::std::fmt::Debug::fmt(parent, __formatter)?;
                __formatter.write_str(",")?;
                ::std::fmt::Debug::fmt(child, __formatter)?;
                __formatter.write_str("}")
            }
        }
    }
}
impl ::std::fmt::Debug for InputScope {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        ::std::fmt::Display::fmt(&self, f)
    }
}
#[derive(Eq, Ord, Clone, Hash, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum Lit {
    LitInt {
        i: crate::ddlog_std::s64
    },
    LitBool {
        b: bool
    },
    LitStr {
        s: crate::internment::istring
    }
}
impl abomonation::Abomonation for Lit{}
::differential_datalog::decl_enum_from_record!(Lit["Lit"]<>, LitInt["LitInt"][1]{[0]i["i"]: crate::ddlog_std::s64}, LitBool["LitBool"][1]{[0]b["b"]: bool}, LitStr["LitStr"][1]{[0]s["s"]: crate::internment::istring});
::differential_datalog::decl_enum_into_record!(Lit<>, LitInt["LitInt"]{i}, LitBool["LitBool"]{b}, LitStr["LitStr"]{s});
::differential_datalog::decl_record_mutator_enum!(Lit<>, LitInt{i: crate::ddlog_std::s64}, LitBool{b: bool}, LitStr{s: crate::internment::istring});
impl ::std::fmt::Display for Lit {
    fn fmt(&self, __formatter: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        match self {
            crate::Lit::LitInt{i} => {
                __formatter.write_str("LitInt{")?;
                ::std::fmt::Debug::fmt(i, __formatter)?;
                __formatter.write_str("}")
            },
            crate::Lit::LitBool{b} => {
                __formatter.write_str("LitBool{")?;
                ::std::fmt::Debug::fmt(b, __formatter)?;
                __formatter.write_str("}")
            },
            crate::Lit::LitStr{s} => {
                __formatter.write_str("LitStr{")?;
                ::std::fmt::Debug::fmt(s, __formatter)?;
                __formatter.write_str("}")
            }
        }
    }
}
impl ::std::fmt::Debug for Lit {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        ::std::fmt::Display::fmt(&self, f)
    }
}
impl ::std::default::Default for Lit {
    fn default() -> Self {
        crate::Lit::LitInt{i : ::std::default::Default::default()}
    }
}
#[derive(Eq, Ord, Clone, Hash, PartialEq, PartialOrd, Default, Serialize, Deserialize)]
pub struct Literal {
    pub expr: crate::ExprId,
    pub lit: crate::Lit
}
impl abomonation::Abomonation for Literal{}
::differential_datalog::decl_struct_from_record!(Literal["Literal"]<>, ["Literal"][2]{[0]expr["expr"]: crate::ExprId, [1]lit["lit"]: crate::Lit});
::differential_datalog::decl_struct_into_record!(Literal, ["Literal"]<>, expr, lit);
::differential_datalog::decl_record_mutator_struct!(Literal, <>, expr: crate::ExprId, lit: crate::Lit);
impl ::std::fmt::Display for Literal {
    fn fmt(&self, __formatter: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        match self {
            crate::Literal{expr,lit} => {
                __formatter.write_str("Literal{")?;
                ::std::fmt::Debug::fmt(expr, __formatter)?;
                __formatter.write_str(",")?;
                ::std::fmt::Debug::fmt(lit, __formatter)?;
                __formatter.write_str("}")
            }
        }
    }
}
impl ::std::fmt::Debug for Literal {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        ::std::fmt::Display::fmt(&self, f)
    }
}
#[derive(Eq, Ord, Clone, Hash, PartialEq, PartialOrd, Default, Serialize, Deserialize)]
pub struct NonexistantFunction {
    pub name: crate::Ident,
    pub invoked: crate::ExprId
}
impl abomonation::Abomonation for NonexistantFunction{}
::differential_datalog::decl_struct_from_record!(NonexistantFunction["NonexistantFunction"]<>, ["NonexistantFunction"][2]{[0]name["name"]: crate::Ident, [1]invoked["invoked"]: crate::ExprId});
::differential_datalog::decl_struct_into_record!(NonexistantFunction, ["NonexistantFunction"]<>, name, invoked);
::differential_datalog::decl_record_mutator_struct!(NonexistantFunction, <>, name: crate::Ident, invoked: crate::ExprId);
impl ::std::fmt::Display for NonexistantFunction {
    fn fmt(&self, __formatter: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        match self {
            crate::NonexistantFunction{name,invoked} => {
                __formatter.write_str("NonexistantFunction{")?;
                ::std::fmt::Debug::fmt(name, __formatter)?;
                __formatter.write_str(",")?;
                ::std::fmt::Debug::fmt(invoked, __formatter)?;
                __formatter.write_str("}")
            }
        }
    }
}
impl ::std::fmt::Debug for NonexistantFunction {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        ::std::fmt::Display::fmt(&self, f)
    }
}
#[derive(Eq, Ord, Clone, Hash, PartialEq, PartialOrd, Default, Serialize, Deserialize)]
pub struct OutOfScopeVar {
    pub variable: crate::Ident,
    pub used: crate::ExprId
}
impl abomonation::Abomonation for OutOfScopeVar{}
::differential_datalog::decl_struct_from_record!(OutOfScopeVar["OutOfScopeVar"]<>, ["OutOfScopeVar"][2]{[0]variable["variable"]: crate::Ident, [1]used["used"]: crate::ExprId});
::differential_datalog::decl_struct_into_record!(OutOfScopeVar, ["OutOfScopeVar"]<>, variable, used);
::differential_datalog::decl_record_mutator_struct!(OutOfScopeVar, <>, variable: crate::Ident, used: crate::ExprId);
impl ::std::fmt::Display for OutOfScopeVar {
    fn fmt(&self, __formatter: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        match self {
            crate::OutOfScopeVar{variable,used} => {
                __formatter.write_str("OutOfScopeVar{")?;
                ::std::fmt::Debug::fmt(variable, __formatter)?;
                __formatter.write_str(",")?;
                ::std::fmt::Debug::fmt(used, __formatter)?;
                __formatter.write_str("}")
            }
        }
    }
}
impl ::std::fmt::Debug for OutOfScopeVar {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        ::std::fmt::Display::fmt(&self, f)
    }
}
pub type Scope = u32;
#[derive(Eq, Ord, Clone, Hash, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum Type {
    Bool,
    Int,
    String,
    Unit,
    Unknown,
    Func {
        args: crate::ddlog_std::Vec<crate::Type>,
        ret: crate::ddlog_std::Ref<crate::Type>
    },
    Poison
}
impl abomonation::Abomonation for Type{}
::differential_datalog::decl_enum_from_record!(Type["Type"]<>, Bool["Bool"][0]{}, Int["Int"][0]{}, String["String"][0]{}, Unit["Unit"][0]{}, Unknown["Unknown"][0]{}, Func["Func"][2]{[0]args["args"]: crate::ddlog_std::Vec<crate::Type>, [1]ret["ret"]: crate::ddlog_std::Ref<crate::Type>}, Poison["Poison"][0]{});
::differential_datalog::decl_enum_into_record!(Type<>, Bool["Bool"]{}, Int["Int"]{}, String["String"]{}, Unit["Unit"]{}, Unknown["Unknown"]{}, Func["Func"]{args, ret}, Poison["Poison"]{});
::differential_datalog::decl_record_mutator_enum!(Type<>, Bool{}, Int{}, String{}, Unit{}, Unknown{}, Func{args: crate::ddlog_std::Vec<crate::Type>, ret: crate::ddlog_std::Ref<crate::Type>}, Poison{});
impl ::std::fmt::Display for Type {
    fn fmt(&self, __formatter: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        match self {
            crate::Type::Bool{} => {
                __formatter.write_str("Bool{")?;
                __formatter.write_str("}")
            },
            crate::Type::Int{} => {
                __formatter.write_str("Int{")?;
                __formatter.write_str("}")
            },
            crate::Type::String{} => {
                __formatter.write_str("String{")?;
                __formatter.write_str("}")
            },
            crate::Type::Unit{} => {
                __formatter.write_str("Unit{")?;
                __formatter.write_str("}")
            },
            crate::Type::Unknown{} => {
                __formatter.write_str("Unknown{")?;
                __formatter.write_str("}")
            },
            crate::Type::Func{args,ret} => {
                __formatter.write_str("Func{")?;
                ::std::fmt::Debug::fmt(args, __formatter)?;
                __formatter.write_str(",")?;
                ::std::fmt::Debug::fmt(ret, __formatter)?;
                __formatter.write_str("}")
            },
            crate::Type::Poison{} => {
                __formatter.write_str("Poison{")?;
                __formatter.write_str("}")
            }
        }
    }
}
impl ::std::fmt::Debug for Type {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        ::std::fmt::Display::fmt(&self, f)
    }
}
impl ::std::default::Default for Type {
    fn default() -> Self {
        crate::Type::Bool{}
    }
}
#[derive(Eq, Ord, Clone, Hash, PartialEq, PartialOrd, Default, Serialize, Deserialize)]
pub struct UninferedExpr {
    pub expr: crate::ExprId
}
impl abomonation::Abomonation for UninferedExpr{}
::differential_datalog::decl_struct_from_record!(UninferedExpr["UninferedExpr"]<>, ["UninferedExpr"][1]{[0]expr["expr"]: crate::ExprId});
::differential_datalog::decl_struct_into_record!(UninferedExpr, ["UninferedExpr"]<>, expr);
::differential_datalog::decl_record_mutator_struct!(UninferedExpr, <>, expr: crate::ExprId);
impl ::std::fmt::Display for UninferedExpr {
    fn fmt(&self, __formatter: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        match self {
            crate::UninferedExpr{expr} => {
                __formatter.write_str("UninferedExpr{")?;
                ::std::fmt::Debug::fmt(expr, __formatter)?;
                __formatter.write_str("}")
            }
        }
    }
}
impl ::std::fmt::Debug for UninferedExpr {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        ::std::fmt::Display::fmt(&self, f)
    }
}
#[derive(Eq, Ord, Clone, Hash, PartialEq, PartialOrd, Default, Serialize, Deserialize)]
pub struct VarDecl {
    pub expr: crate::ExprId,
    pub name: crate::Ident,
    pub val: crate::ExprId
}
impl abomonation::Abomonation for VarDecl{}
::differential_datalog::decl_struct_from_record!(VarDecl["VarDecl"]<>, ["VarDecl"][3]{[0]expr["expr"]: crate::ExprId, [1]name["name"]: crate::Ident, [2]val["val"]: crate::ExprId});
::differential_datalog::decl_struct_into_record!(VarDecl, ["VarDecl"]<>, expr, name, val);
::differential_datalog::decl_record_mutator_struct!(VarDecl, <>, expr: crate::ExprId, name: crate::Ident, val: crate::ExprId);
impl ::std::fmt::Display for VarDecl {
    fn fmt(&self, __formatter: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        match self {
            crate::VarDecl{expr,name,val} => {
                __formatter.write_str("VarDecl{")?;
                ::std::fmt::Debug::fmt(expr, __formatter)?;
                __formatter.write_str(",")?;
                ::std::fmt::Debug::fmt(name, __formatter)?;
                __formatter.write_str(",")?;
                ::std::fmt::Debug::fmt(val, __formatter)?;
                __formatter.write_str("}")
            }
        }
    }
}
impl ::std::fmt::Debug for VarDecl {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        ::std::fmt::Display::fmt(&self, f)
    }
}
#[derive(Eq, Ord, Clone, Hash, PartialEq, PartialOrd, Default, Serialize, Deserialize)]
pub struct Variable {
    pub scope: crate::Scope,
    pub name: crate::Ident,
    pub ty: crate::Type
}
impl abomonation::Abomonation for Variable{}
::differential_datalog::decl_struct_from_record!(Variable["Variable"]<>, ["Variable"][3]{[0]scope["scope"]: crate::Scope, [1]name["name"]: crate::Ident, [2]ty["ty"]: crate::Type});
::differential_datalog::decl_struct_into_record!(Variable, ["Variable"]<>, scope, name, ty);
::differential_datalog::decl_record_mutator_struct!(Variable, <>, scope: crate::Scope, name: crate::Ident, ty: crate::Type);
impl ::std::fmt::Display for Variable {
    fn fmt(&self, __formatter: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        match self {
            crate::Variable{scope,name,ty} => {
                __formatter.write_str("Variable{")?;
                ::std::fmt::Debug::fmt(scope, __formatter)?;
                __formatter.write_str(",")?;
                ::std::fmt::Debug::fmt(name, __formatter)?;
                __formatter.write_str(",")?;
                ::std::fmt::Debug::fmt(ty, __formatter)?;
                __formatter.write_str("}")
            }
        }
    }
}
impl ::std::fmt::Debug for Variable {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        ::std::fmt::Display::fmt(&self, f)
    }
}
pub fn type_of(lit: & crate::Lit) -> crate::Type
{   match (*lit) {
        crate::Lit::LitInt{i: _} => (crate::Type::Int{}),
        crate::Lit::LitBool{b: _} => (crate::Type::Bool{}),
        crate::Lit::LitStr{s: _} => (crate::Type::String{})
    }
}