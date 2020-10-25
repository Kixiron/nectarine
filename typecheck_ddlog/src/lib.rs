#![allow(
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
    clippy::toplevel_ref_arg
)]

use num::bigint::BigInt;
use std::convert::TryFrom;
use std::ops::Deref;
use std::ptr;
use std::result;
use std::sync;

use ordered_float::*;

use differential_dataflow::collection;
use timely::communication;
use timely::dataflow::scopes;
use timely::worker;

use differential_datalog::ddval::*;
use differential_datalog::int::*;
use differential_datalog::program::*;
use differential_datalog::record;
use differential_datalog::record::IntoRecord;
use differential_datalog::record::UpdCmd;
use differential_datalog::uint::*;
use differential_datalog::DDlogConvert;
use num_traits::cast::FromPrimitive;
use num_traits::identities::One;

use fnv::FnvHashMap;

pub use value::*;

pub mod api;
pub mod ovsdb_api;
pub mod update_handler;

use crate::api::updcmd2upd;
use ::types::closure;
use ::types::string_append;
use ::types::string_append_str;

use serde::ser::SerializeTuple;
use serde::Deserialize;
use serde::Deserializer;
use serde::Serialize;
use serde::Serializer;

/// A default implementation of `DDlogConvert` that just forwards calls
/// to generated functions of equal name.
#[derive(Debug)]
pub struct DDlogConverter {}

impl DDlogConvert for DDlogConverter {
    fn relid2name(relId: RelId) -> Option<&'static str> {
        relid2name(relId)
    }

    fn indexid2name(idxId: IdxId) -> Option<&'static str> {
        indexid2name(idxId)
    }

    fn updcmd2upd(upd_cmd: &UpdCmd) -> result::Result<Update<DDValue>, String> {
        updcmd2upd(upd_cmd)
    }
}

/* Wrapper around `Update<DDValue>` type that implements `Serialize` and `Deserialize`
 * traits.  It is currently only used by the distributed_ddlog crate in order to
 * serialize updates before sending them over the network and deserializing them on the
 * way back.  In other scenarios, the user either creates a `Update<DDValue>` type
 * themselves (when using the strongly typed DDlog API) or deserializes `Update<DDValue>`
 * from `Record` using `DDlogConvert::updcmd2upd()`.
 *
 * Why use a wrapper instead of implementing the traits for `Update<DDValue>` directly?
 * `Update<>` and `DDValue` types are both declared in the `differential_datalog` crate,
 * whereas the `Deserialize` implementation is program-specific and must be in one of the
 * generated crates, so we need a wrapper to avoid creating an orphan `impl`.
 *
 * Serialized representation: we currently only serialize `Insert` and `DeleteValue`
 * commands, represented in serialized form as (polarity, relid, value) tuple.  This way
 * the deserializer first reads relid and uses it to decide which value to deserialize
 * next.
 *
 * `impl Serialize` - serializes the value by forwarding `serialize` call to the `DDValue`
 * object (in fact, it is generic and could be in the `differential_datalog` crate, but we
 * keep it here to make it easier to keep it in sync with `Deserialize`).
 *
 * `impl Deserialize` - gets generated in `Compile.hs` using the macro below.  The macro
 * takes a list of `(relid, type)` and generates a match statement that uses type-specific
 * `Deserialize` for each `relid`.
 */
#[derive(Debug)]
pub struct UpdateSerializer(Update<DDValue>);

impl From<Update<DDValue>> for UpdateSerializer {
    fn from(u: Update<DDValue>) -> Self {
        UpdateSerializer(u)
    }
}
impl From<UpdateSerializer> for Update<DDValue> {
    fn from(u: UpdateSerializer) -> Self {
        u.0
    }
}

impl Serialize for UpdateSerializer {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut tup = serializer.serialize_tuple(3)?;
        match &self.0 {
            Update::Insert { relid, v } => {
                tup.serialize_element(&true)?;
                tup.serialize_element(relid)?;
                tup.serialize_element(v)?;
            }
            Update::DeleteValue { relid, v } => {
                tup.serialize_element(&false)?;
                tup.serialize_element(relid)?;
                tup.serialize_element(v)?;
            }
            _ => panic!("Cannot serialize InsertOrUpdate/Modify/DeleteKey update"),
        };
        tup.end()
    }
}

#[macro_export]
macro_rules! decl_update_deserializer {
    ( $n:ty, $(($rel:expr, $typ:ty)),* ) => {
        impl<'de> ::serde::Deserialize<'de> for $n {
            fn deserialize<D: ::serde::Deserializer<'de>>(deserializer: D) -> ::std::result::Result<Self, D::Error> {

                struct UpdateVisitor;

                impl<'de> ::serde::de::Visitor<'de> for UpdateVisitor {
                    type Value = $n;

                    fn expecting(&self, formatter: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                        formatter.write_str("(polarity, relid, value) tuple")
                    }

                    fn visit_seq<A>(self, mut seq: A) -> ::std::result::Result<Self::Value, A::Error>
                    where A: ::serde::de::SeqAccess<'de> {
                        let polarity = seq.next_element::<bool>()?.ok_or_else(|| <A::Error as ::serde::de::Error>::custom("Missing polarity"))?;
                        let relid = seq.next_element::<RelId>()?.ok_or_else(|| <A::Error as ::serde::de::Error>::custom("Missing relation id"))?;
                        match relid {
                            $(
                                $rel => {
                                    let v = seq.next_element::<$typ>()?.ok_or_else(|| <A::Error as ::serde::de::Error>::custom("Missing value"))?.into_ddvalue();
                                    if polarity {
                                        Ok(UpdateSerializer(Update::Insert{relid, v}))
                                    } else {
                                        Ok(UpdateSerializer(Update::DeleteValue{relid, v}))
                                    }
                                },
                            )*
                            _ => {
                                ::std::result::Result::Err(<A::Error as ::serde::de::Error>::custom(format!("Unknown input relation id {}", relid)))
                            }
                        }
                    }
                }

                deserializer.deserialize_tuple(3, UpdateVisitor)
            }
        }
    };
}


decl_update_deserializer!(UpdateSerializer,(0, Value::Application), (1, Value::ApplicationArg), (2, Value::ChildScope), (3, Value::Expression), (4, Value::ExpressionType), (5, Value::FuncArg), (6, Value::Function), (7, Value::Application), (8, Value::ApplicationArg), (9, Value::Expression), (10, Value::FuncArg), (11, Value::Function), (12, Value::InputScope), (13, Value::Literal), (14, Value::VarDecl), (15, Value::InputScope), (16, Value::Literal), (17, Value::NonexistantFunction), (18, Value::OutOfScopeVar), (19, Value::UninferedExpr), (20, Value::VarDecl), (21, Value::Variable));
pub fn prog(__update_cb: Box<dyn CBFn>) -> Program {
    let Application = Relation {
                          name:         "Application".to_string(),
                          input:        true,
                          distinct:     false,
                          caching_mode: CachingMode::Set,
                          key_func:     None,
                          id:           Relations::Application as RelId,
                          rules:        vec![
                              ],
                          arrangements: vec![
                              Arrangement::Map{
                                 name: r###"(Application{.expr=(_0: bit<32>), .func=(_: bit<32>)}: Application) /*join*/"###.to_string(),
                                  afun: &{fn __f(__v: DDValue) -> Option<(DDValue,DDValue)>
                                  {
                                      let __cloned = __v.clone();
                                      match unsafe { Value::Application::from_ddvalue(__v) }.0 {
                                          ::types::Application{expr: ref _0, func: _} => Some(Value::__Bitval32((*_0).clone()).into_ddvalue()),
                                          _ => None
                                      }.map(|x|(x,__cloned))
                                  }
                                  __f},
                                  queryable: false
                              },
                              Arrangement::Map{
                                 name: r###"(Application{.expr=(_: bit<32>), .func=(_0: bit<32>)}: Application) /*join*/"###.to_string(),
                                  afun: &{fn __f(__v: DDValue) -> Option<(DDValue,DDValue)>
                                  {
                                      let __cloned = __v.clone();
                                      match unsafe { Value::Application::from_ddvalue(__v) }.0 {
                                          ::types::Application{expr: _, func: ref _0} => Some(Value::__Bitval32((*_0).clone()).into_ddvalue()),
                                          _ => None
                                      }.map(|x|(x,__cloned))
                                  }
                                  __f},
                                  queryable: false
                              }],
                          change_cb:    None
                      };
    let INPUT_Application = Relation {
                                name:         "INPUT_Application".to_string(),
                                input:        false,
                                distinct:     false,
                                caching_mode: CachingMode::Set,
                                key_func:     None,
                                id:           Relations::INPUT_Application as RelId,
                                rules:        vec![
                                    /* INPUT_Application[x] :- Application[(x: Application)]. */
                                    Rule::CollectionRule {
                                        description: "INPUT_Application[x] :- Application[(x: Application)].".to_string(),
                                        rel: Relations::Application as RelId,
                                        xform: Some(XFormCollection::FilterMap{
                                                        description: "head of INPUT_Application[x] :- Application[(x: Application)]." .to_string(),
                                                        fmfun: &{fn __f(__v: DDValue) -> Option<DDValue>
                                                        {
                                                            let ref x = match unsafe {  Value::Application::from_ddvalue_ref(&__v) }.0 {
                                                                ref x => (*x).clone(),
                                                                _ => return None
                                                            };
                                                            Some(Value::Application((*x).clone()).into_ddvalue())
                                                        }
                                                        __f},
                                                        next: Box::new(None)
                                                    })
                                    }],
                                arrangements: vec![
                                    ],
                                change_cb:    Some(sync::Arc::new(sync::Mutex::new(__update_cb.clone())))
                            };
    let ApplicationArg = Relation {
                             name:         "ApplicationArg".to_string(),
                             input:        true,
                             distinct:     false,
                             caching_mode: CachingMode::Set,
                             key_func:     None,
                             id:           Relations::ApplicationArg as RelId,
                             rules:        vec![
                                 ],
                             arrangements: vec![
                                 ],
                             change_cb:    None
                         };
    let INPUT_ApplicationArg = Relation {
                                   name:         "INPUT_ApplicationArg".to_string(),
                                   input:        false,
                                   distinct:     false,
                                   caching_mode: CachingMode::Set,
                                   key_func:     None,
                                   id:           Relations::INPUT_ApplicationArg as RelId,
                                   rules:        vec![
                                       /* INPUT_ApplicationArg[x] :- ApplicationArg[(x: ApplicationArg)]. */
                                       Rule::CollectionRule {
                                           description: "INPUT_ApplicationArg[x] :- ApplicationArg[(x: ApplicationArg)].".to_string(),
                                           rel: Relations::ApplicationArg as RelId,
                                           xform: Some(XFormCollection::FilterMap{
                                                           description: "head of INPUT_ApplicationArg[x] :- ApplicationArg[(x: ApplicationArg)]." .to_string(),
                                                           fmfun: &{fn __f(__v: DDValue) -> Option<DDValue>
                                                           {
                                                               let ref x = match unsafe {  Value::ApplicationArg::from_ddvalue_ref(&__v) }.0 {
                                                                   ref x => (*x).clone(),
                                                                   _ => return None
                                                               };
                                                               Some(Value::ApplicationArg((*x).clone()).into_ddvalue())
                                                           }
                                                           __f},
                                                           next: Box::new(None)
                                                       })
                                       }],
                                   arrangements: vec![
                                       ],
                                   change_cb:    Some(sync::Arc::new(sync::Mutex::new(__update_cb.clone())))
                               };
    let Expression = Relation {
                         name:         "Expression".to_string(),
                         input:        true,
                         distinct:     false,
                         caching_mode: CachingMode::Set,
                         key_func:     None,
                         id:           Relations::Expression as RelId,
                         rules:        vec![
                             ],
                         arrangements: vec![
                             Arrangement::Map{
                                name: r###"(Expression{.id=(_: bit<32>), .func=(_: bit<32>), .kind=(Var{.v=(_1: internment::Intern<string>)}: ExprKind), .scope=(_0: bit<32>)}: Expression) /*join*/"###.to_string(),
                                 afun: &{fn __f(__v: DDValue) -> Option<(DDValue,DDValue)>
                                 {
                                     let __cloned = __v.clone();
                                     match unsafe { Value::Expression::from_ddvalue(__v) }.0 {
                                         ::types::Expression{id: _, func: _, kind: ::types::ExprKind::Var{v: ref _1}, scope: ref _0} => Some(Value::__Tuple2____Bitval32_internment_Intern____Stringval(((*_0).clone(), (*_1).clone())).into_ddvalue()),
                                         _ => None
                                     }.map(|x|(x,__cloned))
                                 }
                                 __f},
                                 queryable: false
                             },
                             Arrangement::Map{
                                name: r###"(Expression{.id=(_0: bit<32>), .func=(_: bit<32>), .kind=(Var{.v=(_: internment::Intern<string>)}: ExprKind), .scope=(_: bit<32>)}: Expression) /*join*/"###.to_string(),
                                 afun: &{fn __f(__v: DDValue) -> Option<(DDValue,DDValue)>
                                 {
                                     let __cloned = __v.clone();
                                     match unsafe { Value::Expression::from_ddvalue(__v) }.0 {
                                         ::types::Expression{id: ref _0, func: _, kind: ::types::ExprKind::Var{v: _}, scope: _} => Some(Value::__Bitval32((*_0).clone()).into_ddvalue()),
                                         _ => None
                                     }.map(|x|(x,__cloned))
                                 }
                                 __f},
                                 queryable: false
                             },
                             Arrangement::Map{
                                name: r###"(Expression{.id=(_0: bit<32>), .func=(_: bit<32>), .kind=(_: ExprKind), .scope=(_: bit<32>)}: Expression) /*join*/"###.to_string(),
                                 afun: &{fn __f(__v: DDValue) -> Option<(DDValue,DDValue)>
                                 {
                                     let __cloned = __v.clone();
                                     match unsafe { Value::Expression::from_ddvalue(__v) }.0 {
                                         ::types::Expression{id: ref _0, func: _, kind: _, scope: _} => Some(Value::__Bitval32((*_0).clone()).into_ddvalue()),
                                         _ => None
                                     }.map(|x|(x,__cloned))
                                 }
                                 __f},
                                 queryable: false
                             }],
                         change_cb:    None
                     };
    let NonexistantFunction = Relation {
                                  name:         "NonexistantFunction".to_string(),
                                  input:        false,
                                  distinct:     true,
                                  caching_mode: CachingMode::Set,
                                  key_func:     None,
                                  id:           Relations::NonexistantFunction as RelId,
                                  rules:        vec![
                                      /* NonexistantFunction[(NonexistantFunction{.name=name, .invoked=invoked}: NonexistantFunction)] :- Application[(Application{.expr=(invoked: bit<32>), .func=(func: bit<32>)}: Application)], Expression[(Expression{.id=(func: bit<32>), .func=(_: bit<32>), .kind=(Var{.v=(name: internment::Intern<string>)}: ExprKind), .scope=(_: bit<32>)}: Expression)]. */
                                      Rule::ArrangementRule {
                                          description: "NonexistantFunction[(NonexistantFunction{.name=name, .invoked=invoked}: NonexistantFunction)] :- Application[(Application{.expr=(invoked: bit<32>), .func=(func: bit<32>)}: Application)], Expression[(Expression{.id=(func: bit<32>), .func=(_: bit<32>), .kind=(Var{.v=(name: internment::Intern<string>)}: ExprKind), .scope=(_: bit<32>)}: Expression)].".to_string(),
                                          arr: ( Relations::Application as RelId, 1),
                                          xform: XFormArrangement::Join{
                                                     description: "Application[(Application{.expr=(invoked: bit<32>), .func=(func: bit<32>)}: Application)], Expression[(Expression{.id=(func: bit<32>), .func=(_: bit<32>), .kind=(Var{.v=(name: internment::Intern<string>)}: ExprKind), .scope=(_: bit<32>)}: Expression)]".to_string(),
                                                     ffun: None,
                                                     arrangement: (Relations::Expression as RelId,1),
                                                     jfun: &{fn __f(_: &DDValue ,__v1: &DDValue,__v2: &DDValue) -> Option<DDValue>
                                                     {
                                                         let (ref invoked, ref func) = match unsafe {  Value::Application::from_ddvalue_ref(__v1) }.0 {
                                                             ::types::Application{expr: ref invoked, func: ref func} => ((*invoked).clone(), (*func).clone()),
                                                             _ => return None
                                                         };
                                                         let ref name = match unsafe {  Value::Expression::from_ddvalue_ref(__v2) }.0 {
                                                             ::types::Expression{id: _, func: _, kind: ::types::ExprKind::Var{v: ref name}, scope: _} => (*name).clone(),
                                                             _ => return None
                                                         };
                                                         Some(Value::NonexistantFunction((::types::NonexistantFunction{name: (*name).clone(), invoked: (*invoked).clone()})).into_ddvalue())
                                                     }
                                                     __f},
                                                     next: Box::new(None)
                                                 }
                                      }],
                                  arrangements: vec![
                                      ],
                                  change_cb:    Some(sync::Arc::new(sync::Mutex::new(__update_cb.clone())))
                              };
    let INPUT_Expression = Relation {
                               name:         "INPUT_Expression".to_string(),
                               input:        false,
                               distinct:     false,
                               caching_mode: CachingMode::Set,
                               key_func:     None,
                               id:           Relations::INPUT_Expression as RelId,
                               rules:        vec![
                                   /* INPUT_Expression[x] :- Expression[(x: Expression)]. */
                                   Rule::CollectionRule {
                                       description: "INPUT_Expression[x] :- Expression[(x: Expression)].".to_string(),
                                       rel: Relations::Expression as RelId,
                                       xform: Some(XFormCollection::FilterMap{
                                                       description: "head of INPUT_Expression[x] :- Expression[(x: Expression)]." .to_string(),
                                                       fmfun: &{fn __f(__v: DDValue) -> Option<DDValue>
                                                       {
                                                           let ref x = match unsafe {  Value::Expression::from_ddvalue_ref(&__v) }.0 {
                                                               ref x => (*x).clone(),
                                                               _ => return None
                                                           };
                                                           Some(Value::Expression((*x).clone()).into_ddvalue())
                                                       }
                                                       __f},
                                                       next: Box::new(None)
                                                   })
                                   }],
                               arrangements: vec![
                                   ],
                               change_cb:    Some(sync::Arc::new(sync::Mutex::new(__update_cb.clone())))
                           };
    let FuncArg = Relation {
                      name:         "FuncArg".to_string(),
                      input:        true,
                      distinct:     false,
                      caching_mode: CachingMode::Set,
                      key_func:     None,
                      id:           Relations::FuncArg as RelId,
                      rules:        vec![
                          ],
                      arrangements: vec![
                          Arrangement::Map{
                             name: r###"(FuncArg{.func=(_0: bit<32>), .name=(_: internment::Intern<string>), .ty=(_: Type)}: FuncArg) /*join*/"###.to_string(),
                              afun: &{fn __f(__v: DDValue) -> Option<(DDValue,DDValue)>
                              {
                                  let __cloned = __v.clone();
                                  match unsafe { Value::FuncArg::from_ddvalue(__v) }.0 {
                                      ::types::FuncArg{func: ref _0, name: _, ty: _} => Some(Value::__Bitval32((*_0).clone()).into_ddvalue()),
                                      _ => None
                                  }.map(|x|(x,__cloned))
                              }
                              __f},
                              queryable: false
                          }],
                      change_cb:    None
                  };
    let INPUT_FuncArg = Relation {
                            name:         "INPUT_FuncArg".to_string(),
                            input:        false,
                            distinct:     false,
                            caching_mode: CachingMode::Set,
                            key_func:     None,
                            id:           Relations::INPUT_FuncArg as RelId,
                            rules:        vec![
                                /* INPUT_FuncArg[x] :- FuncArg[(x: FuncArg)]. */
                                Rule::CollectionRule {
                                    description: "INPUT_FuncArg[x] :- FuncArg[(x: FuncArg)].".to_string(),
                                    rel: Relations::FuncArg as RelId,
                                    xform: Some(XFormCollection::FilterMap{
                                                    description: "head of INPUT_FuncArg[x] :- FuncArg[(x: FuncArg)]." .to_string(),
                                                    fmfun: &{fn __f(__v: DDValue) -> Option<DDValue>
                                                    {
                                                        let ref x = match unsafe {  Value::FuncArg::from_ddvalue_ref(&__v) }.0 {
                                                            ref x => (*x).clone(),
                                                            _ => return None
                                                        };
                                                        Some(Value::FuncArg((*x).clone()).into_ddvalue())
                                                    }
                                                    __f},
                                                    next: Box::new(None)
                                                })
                                }],
                            arrangements: vec![
                                ],
                            change_cb:    Some(sync::Arc::new(sync::Mutex::new(__update_cb.clone())))
                        };
    let Function = Relation {
                       name:         "Function".to_string(),
                       input:        true,
                       distinct:     false,
                       caching_mode: CachingMode::Set,
                       key_func:     None,
                       id:           Relations::Function as RelId,
                       rules:        vec![
                           ],
                       arrangements: vec![
                           Arrangement::Map{
                              name: r###"(Function{.name=(_: internment::Intern<string>), .id=(_0: bit<32>), .scope=(_: bit<32>), .ret=(_: ddlog_std::Option<Type>)}: Function) /*join*/"###.to_string(),
                               afun: &{fn __f(__v: DDValue) -> Option<(DDValue,DDValue)>
                               {
                                   let __cloned = __v.clone();
                                   match unsafe { Value::Function::from_ddvalue(__v) }.0 {
                                       ::types::Function{name: _, id: ref _0, scope: _, ret: _} => Some(Value::__Bitval32((*_0).clone()).into_ddvalue()),
                                       _ => None
                                   }.map(|x|(x,__cloned))
                               }
                               __f},
                               queryable: false
                           }],
                       change_cb:    None
                   };
    let INPUT_Function = Relation {
                             name:         "INPUT_Function".to_string(),
                             input:        false,
                             distinct:     false,
                             caching_mode: CachingMode::Set,
                             key_func:     None,
                             id:           Relations::INPUT_Function as RelId,
                             rules:        vec![
                                 /* INPUT_Function[x] :- Function[(x: Function)]. */
                                 Rule::CollectionRule {
                                     description: "INPUT_Function[x] :- Function[(x: Function)].".to_string(),
                                     rel: Relations::Function as RelId,
                                     xform: Some(XFormCollection::FilterMap{
                                                     description: "head of INPUT_Function[x] :- Function[(x: Function)]." .to_string(),
                                                     fmfun: &{fn __f(__v: DDValue) -> Option<DDValue>
                                                     {
                                                         let ref x = match unsafe {  Value::Function::from_ddvalue_ref(&__v) }.0 {
                                                             ref x => (*x).clone(),
                                                             _ => return None
                                                         };
                                                         Some(Value::Function((*x).clone()).into_ddvalue())
                                                     }
                                                     __f},
                                                     next: Box::new(None)
                                                 })
                                 }],
                             arrangements: vec![
                                 ],
                             change_cb:    Some(sync::Arc::new(sync::Mutex::new(__update_cb.clone())))
                         };
    let InputScope = Relation {
                         name:         "InputScope".to_string(),
                         input:        true,
                         distinct:     false,
                         caching_mode: CachingMode::Set,
                         key_func:     None,
                         id:           Relations::InputScope as RelId,
                         rules:        vec![
                             ],
                         arrangements: vec![
                             ],
                         change_cb:    None
                     };
    let ChildScope = Relation {
                         name:         "ChildScope".to_string(),
                         input:        false,
                         distinct:     false,
                         caching_mode: CachingMode::Set,
                         key_func:     None,
                         id:           Relations::ChildScope as RelId,
                         rules:        vec![
                             /* ChildScope[(ChildScope{.parent=parent, .child=child}: ChildScope)] :- InputScope[(InputScope{.parent=(parent: bit<32>), .child=(child: bit<32>)}: InputScope)]. */
                             Rule::CollectionRule {
                                 description: "ChildScope[(ChildScope{.parent=parent, .child=child}: ChildScope)] :- InputScope[(InputScope{.parent=(parent: bit<32>), .child=(child: bit<32>)}: InputScope)].".to_string(),
                                 rel: Relations::InputScope as RelId,
                                 xform: Some(XFormCollection::FilterMap{
                                                 description: "head of ChildScope[(ChildScope{.parent=parent, .child=child}: ChildScope)] :- InputScope[(InputScope{.parent=(parent: bit<32>), .child=(child: bit<32>)}: InputScope)]." .to_string(),
                                                 fmfun: &{fn __f(__v: DDValue) -> Option<DDValue>
                                                 {
                                                     let (ref parent, ref child) = match unsafe {  Value::InputScope::from_ddvalue_ref(&__v) }.0 {
                                                         ::types::InputScope{parent: ref parent, child: ref child} => ((*parent).clone(), (*child).clone()),
                                                         _ => return None
                                                     };
                                                     Some(Value::ChildScope((::types::ChildScope{parent: (*parent).clone(), child: (*child).clone()})).into_ddvalue())
                                                 }
                                                 __f},
                                                 next: Box::new(None)
                                             })
                             },
                             /* ChildScope[(ChildScope{.parent=parent, .child=child}: ChildScope)] :- ChildScope[(ChildScope{.parent=(parent: bit<32>), .child=(interum: bit<32>)}: ChildScope)], ChildScope[(ChildScope{.parent=(interum: bit<32>), .child=(child: bit<32>)}: ChildScope)]. */
                             Rule::ArrangementRule {
                                 description: "ChildScope[(ChildScope{.parent=parent, .child=child}: ChildScope)] :- ChildScope[(ChildScope{.parent=(parent: bit<32>), .child=(interum: bit<32>)}: ChildScope)], ChildScope[(ChildScope{.parent=(interum: bit<32>), .child=(child: bit<32>)}: ChildScope)].".to_string(),
                                 arr: ( Relations::ChildScope as RelId, 0),
                                 xform: XFormArrangement::Join{
                                            description: "ChildScope[(ChildScope{.parent=(parent: bit<32>), .child=(interum: bit<32>)}: ChildScope)], ChildScope[(ChildScope{.parent=(interum: bit<32>), .child=(child: bit<32>)}: ChildScope)]".to_string(),
                                            ffun: None,
                                            arrangement: (Relations::ChildScope as RelId,1),
                                            jfun: &{fn __f(_: &DDValue ,__v1: &DDValue,__v2: &DDValue) -> Option<DDValue>
                                            {
                                                let (ref parent, ref interum) = match unsafe {  Value::ChildScope::from_ddvalue_ref(__v1) }.0 {
                                                    ::types::ChildScope{parent: ref parent, child: ref interum} => ((*parent).clone(), (*interum).clone()),
                                                    _ => return None
                                                };
                                                let ref child = match unsafe {  Value::ChildScope::from_ddvalue_ref(__v2) }.0 {
                                                    ::types::ChildScope{parent: _, child: ref child} => (*child).clone(),
                                                    _ => return None
                                                };
                                                Some(Value::ChildScope((::types::ChildScope{parent: (*parent).clone(), child: (*child).clone()})).into_ddvalue())
                                            }
                                            __f},
                                            next: Box::new(None)
                                        }
                             }],
                         arrangements: vec![
                             Arrangement::Map{
                                name: r###"(ChildScope{.parent=(_: bit<32>), .child=(_0: bit<32>)}: ChildScope) /*join*/"###.to_string(),
                                 afun: &{fn __f(__v: DDValue) -> Option<(DDValue,DDValue)>
                                 {
                                     let __cloned = __v.clone();
                                     match unsafe { Value::ChildScope::from_ddvalue(__v) }.0 {
                                         ::types::ChildScope{parent: _, child: ref _0} => Some(Value::__Bitval32((*_0).clone()).into_ddvalue()),
                                         _ => None
                                     }.map(|x|(x,__cloned))
                                 }
                                 __f},
                                 queryable: false
                             },
                             Arrangement::Map{
                                name: r###"(ChildScope{.parent=(_0: bit<32>), .child=(_: bit<32>)}: ChildScope) /*join*/"###.to_string(),
                                 afun: &{fn __f(__v: DDValue) -> Option<(DDValue,DDValue)>
                                 {
                                     let __cloned = __v.clone();
                                     match unsafe { Value::ChildScope::from_ddvalue(__v) }.0 {
                                         ::types::ChildScope{parent: ref _0, child: _} => Some(Value::__Bitval32((*_0).clone()).into_ddvalue()),
                                         _ => None
                                     }.map(|x|(x,__cloned))
                                 }
                                 __f},
                                 queryable: false
                             }],
                         change_cb:    Some(sync::Arc::new(sync::Mutex::new(__update_cb.clone())))
                     };
    let INPUT_InputScope = Relation {
                               name:         "INPUT_InputScope".to_string(),
                               input:        false,
                               distinct:     false,
                               caching_mode: CachingMode::Set,
                               key_func:     None,
                               id:           Relations::INPUT_InputScope as RelId,
                               rules:        vec![
                                   /* INPUT_InputScope[x] :- InputScope[(x: InputScope)]. */
                                   Rule::CollectionRule {
                                       description: "INPUT_InputScope[x] :- InputScope[(x: InputScope)].".to_string(),
                                       rel: Relations::InputScope as RelId,
                                       xform: Some(XFormCollection::FilterMap{
                                                       description: "head of INPUT_InputScope[x] :- InputScope[(x: InputScope)]." .to_string(),
                                                       fmfun: &{fn __f(__v: DDValue) -> Option<DDValue>
                                                       {
                                                           let ref x = match unsafe {  Value::InputScope::from_ddvalue_ref(&__v) }.0 {
                                                               ref x => (*x).clone(),
                                                               _ => return None
                                                           };
                                                           Some(Value::InputScope((*x).clone()).into_ddvalue())
                                                       }
                                                       __f},
                                                       next: Box::new(None)
                                                   })
                                   }],
                               arrangements: vec![
                                   ],
                               change_cb:    Some(sync::Arc::new(sync::Mutex::new(__update_cb.clone())))
                           };
    let Literal = Relation {
                      name:         "Literal".to_string(),
                      input:        true,
                      distinct:     false,
                      caching_mode: CachingMode::Set,
                      key_func:     None,
                      id:           Relations::Literal as RelId,
                      rules:        vec![
                          ],
                      arrangements: vec![
                          ],
                      change_cb:    None
                  };
    let INPUT_Literal = Relation {
                            name:         "INPUT_Literal".to_string(),
                            input:        false,
                            distinct:     false,
                            caching_mode: CachingMode::Set,
                            key_func:     None,
                            id:           Relations::INPUT_Literal as RelId,
                            rules:        vec![
                                /* INPUT_Literal[x] :- Literal[(x: Literal)]. */
                                Rule::CollectionRule {
                                    description: "INPUT_Literal[x] :- Literal[(x: Literal)].".to_string(),
                                    rel: Relations::Literal as RelId,
                                    xform: Some(XFormCollection::FilterMap{
                                                    description: "head of INPUT_Literal[x] :- Literal[(x: Literal)]." .to_string(),
                                                    fmfun: &{fn __f(__v: DDValue) -> Option<DDValue>
                                                    {
                                                        let ref x = match unsafe {  Value::Literal::from_ddvalue_ref(&__v) }.0 {
                                                            ref x => (*x).clone(),
                                                            _ => return None
                                                        };
                                                        Some(Value::Literal((*x).clone()).into_ddvalue())
                                                    }
                                                    __f},
                                                    next: Box::new(None)
                                                })
                                }],
                            arrangements: vec![
                                ],
                            change_cb:    Some(sync::Arc::new(sync::Mutex::new(__update_cb.clone())))
                        };
    let VarDecl = Relation {
                      name:         "VarDecl".to_string(),
                      input:        true,
                      distinct:     false,
                      caching_mode: CachingMode::Set,
                      key_func:     None,
                      id:           Relations::VarDecl as RelId,
                      rules:        vec![
                          ],
                      arrangements: vec![
                          Arrangement::Map{
                             name: r###"(VarDecl{.expr=(_: bit<32>), .name=(_: internment::Intern<string>), .val=(_0: bit<32>)}: VarDecl) /*join*/"###.to_string(),
                              afun: &{fn __f(__v: DDValue) -> Option<(DDValue,DDValue)>
                              {
                                  let __cloned = __v.clone();
                                  match unsafe { Value::VarDecl::from_ddvalue(__v) }.0 {
                                      ::types::VarDecl{expr: _, name: _, val: ref _0} => Some(Value::__Bitval32((*_0).clone()).into_ddvalue()),
                                      _ => None
                                  }.map(|x|(x,__cloned))
                              }
                              __f},
                              queryable: false
                          },
                          Arrangement::Map{
                             name: r###"(VarDecl{.expr=(_0: bit<32>), .name=(_: internment::Intern<string>), .val=(_: bit<32>)}: VarDecl) /*join*/"###.to_string(),
                              afun: &{fn __f(__v: DDValue) -> Option<(DDValue,DDValue)>
                              {
                                  let __cloned = __v.clone();
                                  match unsafe { Value::VarDecl::from_ddvalue(__v) }.0 {
                                      ::types::VarDecl{expr: ref _0, name: _, val: _} => Some(Value::__Bitval32((*_0).clone()).into_ddvalue()),
                                      _ => None
                                  }.map(|x|(x,__cloned))
                              }
                              __f},
                              queryable: false
                          }],
                      change_cb:    None
                  };
    let ExpressionType = Relation {
                             name:         "ExpressionType".to_string(),
                             input:        false,
                             distinct:     false,
                             caching_mode: CachingMode::Set,
                             key_func:     None,
                             id:           Relations::ExpressionType as RelId,
                             rules:        vec![
                                 /* ExpressionType[(ExpressionType{.expr=expr, .ty=ty}: ExpressionType)] :- Literal[(Literal{.expr=(expr: bit<32>), .lit=(lit: Lit)}: Literal)], ((var ty: Type) = (type_of(lit))). */
                                 Rule::CollectionRule {
                                     description: "ExpressionType[(ExpressionType{.expr=expr, .ty=ty}: ExpressionType)] :- Literal[(Literal{.expr=(expr: bit<32>), .lit=(lit: Lit)}: Literal)], ((var ty: Type) = (type_of(lit))).".to_string(),
                                     rel: Relations::Literal as RelId,
                                     xform: Some(XFormCollection::FilterMap{
                                                     description: "head of ExpressionType[(ExpressionType{.expr=expr, .ty=ty}: ExpressionType)] :- Literal[(Literal{.expr=(expr: bit<32>), .lit=(lit: Lit)}: Literal)], ((var ty: Type) = (type_of(lit)))." .to_string(),
                                                     fmfun: &{fn __f(__v: DDValue) -> Option<DDValue>
                                                     {
                                                         let (ref expr, ref lit) = match unsafe {  Value::Literal::from_ddvalue_ref(&__v) }.0 {
                                                             ::types::Literal{expr: ref expr, lit: ref lit} => ((*expr).clone(), (*lit).clone()),
                                                             _ => return None
                                                         };
                                                         let ref ty: ::types::Type = match ::types::type_of(lit) {
                                                             ty => ty,
                                                             _ => return None
                                                         };
                                                         Some(Value::ExpressionType((::types::ExpressionType{expr: (*expr).clone(), ty: (*ty).clone()})).into_ddvalue())
                                                     }
                                                     __f},
                                                     next: Box::new(None)
                                                 })
                                 },
                                 /* ExpressionType[(ExpressionType{.expr=expr, .ty=ty}: ExpressionType)] :- VarDecl[(VarDecl{.expr=(expr: bit<32>), .name=(_: internment::Intern<string>), .val=(val: bit<32>)}: VarDecl)], ExpressionType[(ExpressionType{.expr=(val: bit<32>), .ty=(ty: Type)}: ExpressionType)]. */
                                 Rule::ArrangementRule {
                                     description: "ExpressionType[(ExpressionType{.expr=expr, .ty=ty}: ExpressionType)] :- VarDecl[(VarDecl{.expr=(expr: bit<32>), .name=(_: internment::Intern<string>), .val=(val: bit<32>)}: VarDecl)], ExpressionType[(ExpressionType{.expr=(val: bit<32>), .ty=(ty: Type)}: ExpressionType)].".to_string(),
                                     arr: ( Relations::VarDecl as RelId, 0),
                                     xform: XFormArrangement::Join{
                                                description: "VarDecl[(VarDecl{.expr=(expr: bit<32>), .name=(_: internment::Intern<string>), .val=(val: bit<32>)}: VarDecl)], ExpressionType[(ExpressionType{.expr=(val: bit<32>), .ty=(ty: Type)}: ExpressionType)]".to_string(),
                                                ffun: None,
                                                arrangement: (Relations::ExpressionType as RelId,0),
                                                jfun: &{fn __f(_: &DDValue ,__v1: &DDValue,__v2: &DDValue) -> Option<DDValue>
                                                {
                                                    let (ref expr, ref val) = match unsafe {  Value::VarDecl::from_ddvalue_ref(__v1) }.0 {
                                                        ::types::VarDecl{expr: ref expr, name: _, val: ref val} => ((*expr).clone(), (*val).clone()),
                                                        _ => return None
                                                    };
                                                    let ref ty = match unsafe {  Value::ExpressionType::from_ddvalue_ref(__v2) }.0 {
                                                        ::types::ExpressionType{expr: _, ty: ref ty} => (*ty).clone(),
                                                        _ => return None
                                                    };
                                                    Some(Value::ExpressionType((::types::ExpressionType{expr: (*expr).clone(), ty: (*ty).clone()})).into_ddvalue())
                                                }
                                                __f},
                                                next: Box::new(None)
                                            }
                                 },
                                 /* ExpressionType[(ExpressionType{.expr=expr, .ty=ty}: ExpressionType)] :- Expression[(Expression{.id=(expr: bit<32>), .func=(_: bit<32>), .kind=(Var{.v=(v: internment::Intern<string>)}: ExprKind), .scope=(scope: bit<32>)}: Expression)], Variable[(Variable{.scope=(scope: bit<32>), .name=(v: internment::Intern<string>), .ty=(ty: Type)}: Variable)]. */
                                 Rule::ArrangementRule {
                                     description: "ExpressionType[(ExpressionType{.expr=expr, .ty=ty}: ExpressionType)] :- Expression[(Expression{.id=(expr: bit<32>), .func=(_: bit<32>), .kind=(Var{.v=(v: internment::Intern<string>)}: ExprKind), .scope=(scope: bit<32>)}: Expression)], Variable[(Variable{.scope=(scope: bit<32>), .name=(v: internment::Intern<string>), .ty=(ty: Type)}: Variable)].".to_string(),
                                     arr: ( Relations::Expression as RelId, 0),
                                     xform: XFormArrangement::Join{
                                                description: "Expression[(Expression{.id=(expr: bit<32>), .func=(_: bit<32>), .kind=(Var{.v=(v: internment::Intern<string>)}: ExprKind), .scope=(scope: bit<32>)}: Expression)], Variable[(Variable{.scope=(scope: bit<32>), .name=(v: internment::Intern<string>), .ty=(ty: Type)}: Variable)]".to_string(),
                                                ffun: None,
                                                arrangement: (Relations::Variable as RelId,0),
                                                jfun: &{fn __f(_: &DDValue ,__v1: &DDValue,__v2: &DDValue) -> Option<DDValue>
                                                {
                                                    let (ref expr, ref v, ref scope) = match unsafe {  Value::Expression::from_ddvalue_ref(__v1) }.0 {
                                                        ::types::Expression{id: ref expr, func: _, kind: ::types::ExprKind::Var{v: ref v}, scope: ref scope} => ((*expr).clone(), (*v).clone(), (*scope).clone()),
                                                        _ => return None
                                                    };
                                                    let ref ty = match unsafe {  Value::Variable::from_ddvalue_ref(__v2) }.0 {
                                                        ::types::Variable{scope: _, name: _, ty: ref ty} => (*ty).clone(),
                                                        _ => return None
                                                    };
                                                    Some(Value::ExpressionType((::types::ExpressionType{expr: (*expr).clone(), ty: (*ty).clone()})).into_ddvalue())
                                                }
                                                __f},
                                                next: Box::new(None)
                                            }
                                 },
                                 /* ExpressionType[(ExpressionType{.expr=expr, .ty=ty}: ExpressionType)] :- Application[(Application{.expr=(expr: bit<32>), .func=(func: bit<32>)}: Application)], Expression[(Expression{.id=(expr: bit<32>), .func=(_: bit<32>), .kind=(Var{.v=(name: internment::Intern<string>)}: ExprKind), .scope=(scope: bit<32>)}: Expression)], Variable[(Variable{.scope=(scope: bit<32>), .name=(name: internment::Intern<string>), .ty=(Func{.args=(_: ddlog_std::Vec<Type>), .ret=(ret: ddlog_std::Ref<Type>)}: Type)}: Variable)], ((var ty: Type) = ((ddlog_std::deref: function(ddlog_std::Ref<Type>):Type)(ret))). */
                                 Rule::ArrangementRule {
                                     description: "ExpressionType[(ExpressionType{.expr=expr, .ty=ty}: ExpressionType)] :- Application[(Application{.expr=(expr: bit<32>), .func=(func: bit<32>)}: Application)], Expression[(Expression{.id=(expr: bit<32>), .func=(_: bit<32>), .kind=(Var{.v=(name: internment::Intern<string>)}: ExprKind), .scope=(scope: bit<32>)}: Expression)], Variable[(Variable{.scope=(scope: bit<32>), .name=(name: internment::Intern<string>), .ty=(Func{.args=(_: ddlog_std::Vec<Type>), .ret=(ret: ddlog_std::Ref<Type>)}: Type)}: Variable)], ((var ty: Type) = ((ddlog_std::deref: function(ddlog_std::Ref<Type>):Type)(ret))).".to_string(),
                                     arr: ( Relations::Application as RelId, 0),
                                     xform: XFormArrangement::Join{
                                                description: "Application[(Application{.expr=(expr: bit<32>), .func=(func: bit<32>)}: Application)], Expression[(Expression{.id=(expr: bit<32>), .func=(_: bit<32>), .kind=(Var{.v=(name: internment::Intern<string>)}: ExprKind), .scope=(scope: bit<32>)}: Expression)]".to_string(),
                                                ffun: None,
                                                arrangement: (Relations::Expression as RelId,1),
                                                jfun: &{fn __f(_: &DDValue ,__v1: &DDValue,__v2: &DDValue) -> Option<DDValue>
                                                {
                                                    let (ref expr, ref func) = match unsafe {  Value::Application::from_ddvalue_ref(__v1) }.0 {
                                                        ::types::Application{expr: ref expr, func: ref func} => ((*expr).clone(), (*func).clone()),
                                                        _ => return None
                                                    };
                                                    let (ref name, ref scope) = match unsafe {  Value::Expression::from_ddvalue_ref(__v2) }.0 {
                                                        ::types::Expression{id: _, func: _, kind: ::types::ExprKind::Var{v: ref name}, scope: ref scope} => ((*name).clone(), (*scope).clone()),
                                                        _ => return None
                                                    };
                                                    Some(Value::__Tuple3____Bitval32_internment_Intern____Stringval___Bitval32(((*expr).clone(), (*name).clone(), (*scope).clone())).into_ddvalue())
                                                }
                                                __f},
                                                next: Box::new(Some(XFormCollection::Arrange {
                                                                        description: "arrange Application[(Application{.expr=(expr: bit<32>), .func=(func: bit<32>)}: Application)], Expression[(Expression{.id=(expr: bit<32>), .func=(_: bit<32>), .kind=(Var{.v=(name: internment::Intern<string>)}: ExprKind), .scope=(scope: bit<32>)}: Expression)] by (scope, name)" .to_string(),
                                                                        afun: &{fn __f(__v: DDValue) -> Option<(DDValue,DDValue)>
                                                                        {
                                                                            let (ref expr, ref name, ref scope) = unsafe { Value::__Tuple3____Bitval32_internment_Intern____Stringval___Bitval32::from_ddvalue_ref( &__v ) }.0;
                                                                            Some((Value::__Tuple2____Bitval32_internment_Intern____Stringval(((*scope).clone(), (*name).clone())).into_ddvalue(), Value::__Bitval32((*expr).clone()).into_ddvalue()))
                                                                        }
                                                                        __f},
                                                                        next: Box::new(XFormArrangement::Join{
                                                                                           description: "Application[(Application{.expr=(expr: bit<32>), .func=(func: bit<32>)}: Application)], Expression[(Expression{.id=(expr: bit<32>), .func=(_: bit<32>), .kind=(Var{.v=(name: internment::Intern<string>)}: ExprKind), .scope=(scope: bit<32>)}: Expression)], Variable[(Variable{.scope=(scope: bit<32>), .name=(name: internment::Intern<string>), .ty=(Func{.args=(_: ddlog_std::Vec<Type>), .ret=(ret: ddlog_std::Ref<Type>)}: Type)}: Variable)]".to_string(),
                                                                                           ffun: None,
                                                                                           arrangement: (Relations::Variable as RelId,1),
                                                                                           jfun: &{fn __f(_: &DDValue ,__v1: &DDValue,__v2: &DDValue) -> Option<DDValue>
                                                                                           {
                                                                                               let ref expr = unsafe { Value::__Bitval32::from_ddvalue_ref( __v1 ) }.0;
                                                                                               let ref ret = match unsafe {  Value::Variable::from_ddvalue_ref(__v2) }.0 {
                                                                                                   ::types::Variable{scope: _, name: _, ty: ::types::Type::Func{args: _, ret: ref ret}} => (*ret).clone(),
                                                                                                   _ => return None
                                                                                               };
                                                                                               let ref ty: ::types::Type = match (*::types::ddlog_std::deref(ret)).clone() {
                                                                                                   ty => ty,
                                                                                                   _ => return None
                                                                                               };
                                                                                               Some(Value::ExpressionType((::types::ExpressionType{expr: (*expr).clone(), ty: (*ty).clone()})).into_ddvalue())
                                                                                           }
                                                                                           __f},
                                                                                           next: Box::new(None)
                                                                                       })
                                                                    }))
                                            }
                                 }],
                             arrangements: vec![
                                 Arrangement::Map{
                                    name: r###"(ExpressionType{.expr=(_0: bit<32>), .ty=(_: Type)}: ExpressionType) /*join*/"###.to_string(),
                                     afun: &{fn __f(__v: DDValue) -> Option<(DDValue,DDValue)>
                                     {
                                         let __cloned = __v.clone();
                                         match unsafe { Value::ExpressionType::from_ddvalue(__v) }.0 {
                                             ::types::ExpressionType{expr: ref _0, ty: _} => Some(Value::__Bitval32((*_0).clone()).into_ddvalue()),
                                             _ => None
                                         }.map(|x|(x,__cloned))
                                     }
                                     __f},
                                     queryable: false
                                 },
                                 Arrangement::Set{
                                     name: r###"(ExpressionType{.expr=(_0: bit<32>), .ty=(_: Type)}: ExpressionType) /*antijoin*/"###.to_string(),
                                     fmfun: &{fn __f(__v: DDValue) -> Option<DDValue>
                                     {
                                         match unsafe { Value::ExpressionType::from_ddvalue(__v) }.0 {
                                             ::types::ExpressionType{expr: ref _0, ty: _} => Some(Value::__Bitval32((*_0).clone()).into_ddvalue()),
                                             _ => None
                                         }
                                     }
                                     __f},
                                     distinct: true
                                 },
                                 Arrangement::Set{
                                     name: r###"(ExpressionType{.expr=(_0: bit<32>), .ty=(Poison{}: Type)}: ExpressionType) /*semijoin*/"###.to_string(),
                                     fmfun: &{fn __f(__v: DDValue) -> Option<DDValue>
                                     {
                                         match unsafe { Value::ExpressionType::from_ddvalue(__v) }.0 {
                                             ::types::ExpressionType{expr: ref _0, ty: ::types::Type::Poison{}} => Some(Value::__Bitval32((*_0).clone()).into_ddvalue()),
                                             _ => None
                                         }
                                     }
                                     __f},
                                     distinct: false
                                 }],
                             change_cb:    Some(sync::Arc::new(sync::Mutex::new(__update_cb.clone())))
                         };
    let Variable = Relation {
                       name:         "Variable".to_string(),
                       input:        false,
                       distinct:     false,
                       caching_mode: CachingMode::Set,
                       key_func:     None,
                       id:           Relations::Variable as RelId,
                       rules:        vec![
                           /* Variable[(Variable{.scope=scope, .name=name, .ty=ty}: Variable)] :- VarDecl[(VarDecl{.expr=(expr: bit<32>), .name=(name: internment::Intern<string>), .val=(val: bit<32>)}: VarDecl)], Expression[(Expression{.id=(expr: bit<32>), .func=(_: bit<32>), .kind=(_: ExprKind), .scope=(scope: bit<32>)}: Expression)], ExpressionType[(ExpressionType{.expr=(val: bit<32>), .ty=(ty: Type)}: ExpressionType)]. */
                           Rule::ArrangementRule {
                               description: "Variable[(Variable{.scope=scope, .name=name, .ty=ty}: Variable)] :- VarDecl[(VarDecl{.expr=(expr: bit<32>), .name=(name: internment::Intern<string>), .val=(val: bit<32>)}: VarDecl)], Expression[(Expression{.id=(expr: bit<32>), .func=(_: bit<32>), .kind=(_: ExprKind), .scope=(scope: bit<32>)}: Expression)], ExpressionType[(ExpressionType{.expr=(val: bit<32>), .ty=(ty: Type)}: ExpressionType)].".to_string(),
                               arr: ( Relations::VarDecl as RelId, 1),
                               xform: XFormArrangement::Join{
                                          description: "VarDecl[(VarDecl{.expr=(expr: bit<32>), .name=(name: internment::Intern<string>), .val=(val: bit<32>)}: VarDecl)], Expression[(Expression{.id=(expr: bit<32>), .func=(_: bit<32>), .kind=(_: ExprKind), .scope=(scope: bit<32>)}: Expression)]".to_string(),
                                          ffun: None,
                                          arrangement: (Relations::Expression as RelId,2),
                                          jfun: &{fn __f(_: &DDValue ,__v1: &DDValue,__v2: &DDValue) -> Option<DDValue>
                                          {
                                              let (ref expr, ref name, ref val) = match unsafe {  Value::VarDecl::from_ddvalue_ref(__v1) }.0 {
                                                  ::types::VarDecl{expr: ref expr, name: ref name, val: ref val} => ((*expr).clone(), (*name).clone(), (*val).clone()),
                                                  _ => return None
                                              };
                                              let ref scope = match unsafe {  Value::Expression::from_ddvalue_ref(__v2) }.0 {
                                                  ::types::Expression{id: _, func: _, kind: _, scope: ref scope} => (*scope).clone(),
                                                  _ => return None
                                              };
                                              Some(Value::__Tuple3__internment_Intern____Stringval___Bitval32___Bitval32(((*name).clone(), (*val).clone(), (*scope).clone())).into_ddvalue())
                                          }
                                          __f},
                                          next: Box::new(Some(XFormCollection::Arrange {
                                                                  description: "arrange VarDecl[(VarDecl{.expr=(expr: bit<32>), .name=(name: internment::Intern<string>), .val=(val: bit<32>)}: VarDecl)], Expression[(Expression{.id=(expr: bit<32>), .func=(_: bit<32>), .kind=(_: ExprKind), .scope=(scope: bit<32>)}: Expression)] by (val)" .to_string(),
                                                                  afun: &{fn __f(__v: DDValue) -> Option<(DDValue,DDValue)>
                                                                  {
                                                                      let (ref name, ref val, ref scope) = unsafe { Value::__Tuple3__internment_Intern____Stringval___Bitval32___Bitval32::from_ddvalue_ref( &__v ) }.0;
                                                                      Some((Value::__Bitval32((*val).clone()).into_ddvalue(), Value::__Tuple2__internment_Intern____Stringval___Bitval32(((*name).clone(), (*scope).clone())).into_ddvalue()))
                                                                  }
                                                                  __f},
                                                                  next: Box::new(XFormArrangement::Join{
                                                                                     description: "VarDecl[(VarDecl{.expr=(expr: bit<32>), .name=(name: internment::Intern<string>), .val=(val: bit<32>)}: VarDecl)], Expression[(Expression{.id=(expr: bit<32>), .func=(_: bit<32>), .kind=(_: ExprKind), .scope=(scope: bit<32>)}: Expression)], ExpressionType[(ExpressionType{.expr=(val: bit<32>), .ty=(ty: Type)}: ExpressionType)]".to_string(),
                                                                                     ffun: None,
                                                                                     arrangement: (Relations::ExpressionType as RelId,0),
                                                                                     jfun: &{fn __f(_: &DDValue ,__v1: &DDValue,__v2: &DDValue) -> Option<DDValue>
                                                                                     {
                                                                                         let (ref name, ref scope) = unsafe { Value::__Tuple2__internment_Intern____Stringval___Bitval32::from_ddvalue_ref( __v1 ) }.0;
                                                                                         let ref ty = match unsafe {  Value::ExpressionType::from_ddvalue_ref(__v2) }.0 {
                                                                                             ::types::ExpressionType{expr: _, ty: ref ty} => (*ty).clone(),
                                                                                             _ => return None
                                                                                         };
                                                                                         Some(Value::Variable((::types::Variable{scope: (*scope).clone(), name: (*name).clone(), ty: (*ty).clone()})).into_ddvalue())
                                                                                     }
                                                                                     __f},
                                                                                     next: Box::new(None)
                                                                                 })
                                                              }))
                                      }
                           },
                           /* Variable[(Variable{.scope=scope, .name=name, .ty=ty}: Variable)] :- Function[(Function{.name=(name: internment::Intern<string>), .id=(func: bit<32>), .scope=(scope: bit<32>), .ret=(func_ret: ddlog_std::Option<Type>)}: Function)], ((var ret: ddlog_std::Ref<Type>) = ((ddlog_std::ref_new: function(Type):ddlog_std::Ref<Type>)(((ddlog_std::unwrap_or: function(ddlog_std::Option<Type>, Type):Type)(func_ret, (Unknown{}: Type)))))), FuncArg[(FuncArg{.func=(func: bit<32>), .name=(_: internment::Intern<string>), .ty=(ty: Type)}: FuncArg)], var __group = ty.group_by((func, ret, scope, name)), ((var args: ddlog_std::Vec<Type>) = ((ddlog_std::group_to_vec: function(ddlog_std::Group<(bit<32>, ddlog_std::Ref<Type>, bit<32>, internment::Intern<string>),Type>):ddlog_std::Vec<Type>)(__group))), ((var ty: Type) = (Func{.args=args, .ret=ret}: Type)). */
                           Rule::CollectionRule {
                               description: "Variable[(Variable{.scope=scope, .name=name, .ty=ty}: Variable)] :- Function[(Function{.name=(name: internment::Intern<string>), .id=(func: bit<32>), .scope=(scope: bit<32>), .ret=(func_ret: ddlog_std::Option<Type>)}: Function)], ((var ret: ddlog_std::Ref<Type>) = ((ddlog_std::ref_new: function(Type):ddlog_std::Ref<Type>)(((ddlog_std::unwrap_or: function(ddlog_std::Option<Type>, Type):Type)(func_ret, (Unknown{}: Type)))))), FuncArg[(FuncArg{.func=(func: bit<32>), .name=(_: internment::Intern<string>), .ty=(ty: Type)}: FuncArg)], var __group = ty.group_by((func, ret, scope, name)), ((var args: ddlog_std::Vec<Type>) = ((ddlog_std::group_to_vec: function(ddlog_std::Group<(bit<32>, ddlog_std::Ref<Type>, bit<32>, internment::Intern<string>),Type>):ddlog_std::Vec<Type>)(__group))), ((var ty: Type) = (Func{.args=args, .ret=ret}: Type)).".to_string(),
                               rel: Relations::Function as RelId,
                               xform: Some(XFormCollection::Arrange {
                                               description: "arrange Function[(Function{.name=(name: internment::Intern<string>), .id=(func: bit<32>), .scope=(scope: bit<32>), .ret=(func_ret: ddlog_std::Option<Type>)}: Function)] by (func)" .to_string(),
                                               afun: &{fn __f(__v: DDValue) -> Option<(DDValue,DDValue)>
                                               {
                                                   let (ref name, ref func, ref scope, ref func_ret) = match unsafe {  Value::Function::from_ddvalue_ref(&__v) }.0 {
                                                       ::types::Function{name: ref name, id: ref func, scope: ref scope, ret: ref func_ret} => ((*name).clone(), (*func).clone(), (*scope).clone(), (*func_ret).clone()),
                                                       _ => return None
                                                   };
                                                   let ref ret: ::types::ddlog_std::Ref<::types::Type> = match ::types::ddlog_std::ref_new((&::types::ddlog_std::unwrap_or_ddlog_std_Option__A_A_A(func_ret, (&(::types::Type::Unknown{}))))) {
                                                       ret => ret,
                                                       _ => return None
                                                   };
                                                   Some((Value::__Bitval32((*func).clone()).into_ddvalue(), Value::__Tuple5__internment_Intern____Stringval___Bitval32___Bitval32_ddlog_std_Option__Type_ddlog_std_Ref__Type(((*name).clone(), (*func).clone(), (*scope).clone(), (*func_ret).clone(), (*ret).clone())).into_ddvalue()))
                                               }
                                               __f},
                                               next: Box::new(XFormArrangement::Join{
                                                                  description: "Function[(Function{.name=(name: internment::Intern<string>), .id=(func: bit<32>), .scope=(scope: bit<32>), .ret=(func_ret: ddlog_std::Option<Type>)}: Function)], ((var ret: ddlog_std::Ref<Type>) = ((ddlog_std::ref_new: function(Type):ddlog_std::Ref<Type>)(((ddlog_std::unwrap_or: function(ddlog_std::Option<Type>, Type):Type)(func_ret, (Unknown{}: Type)))))), FuncArg[(FuncArg{.func=(func: bit<32>), .name=(_: internment::Intern<string>), .ty=(ty: Type)}: FuncArg)]".to_string(),
                                                                  ffun: None,
                                                                  arrangement: (Relations::FuncArg as RelId,0),
                                                                  jfun: &{fn __f(_: &DDValue ,__v1: &DDValue,__v2: &DDValue) -> Option<DDValue>
                                                                  {
                                                                      let (ref name, ref func, ref scope, ref func_ret, ref ret) = unsafe { Value::__Tuple5__internment_Intern____Stringval___Bitval32___Bitval32_ddlog_std_Option__Type_ddlog_std_Ref__Type::from_ddvalue_ref( __v1 ) }.0;
                                                                      let ref ty = match unsafe {  Value::FuncArg::from_ddvalue_ref(__v2) }.0 {
                                                                          ::types::FuncArg{func: _, name: _, ty: ref ty} => (*ty).clone(),
                                                                          _ => return None
                                                                      };
                                                                      Some(Value::__Tuple6__internment_Intern____Stringval___Bitval32___Bitval32_ddlog_std_Option__Type_ddlog_std_Ref__Type_Type(((*name).clone(), (*func).clone(), (*scope).clone(), (*func_ret).clone(), (*ret).clone(), (*ty).clone())).into_ddvalue())
                                                                  }
                                                                  __f},
                                                                  next: Box::new(Some(XFormCollection::Arrange {
                                                                                          description: "arrange Function[(Function{.name=(name: internment::Intern<string>), .id=(func: bit<32>), .scope=(scope: bit<32>), .ret=(func_ret: ddlog_std::Option<Type>)}: Function)], ((var ret: ddlog_std::Ref<Type>) = ((ddlog_std::ref_new: function(Type):ddlog_std::Ref<Type>)(((ddlog_std::unwrap_or: function(ddlog_std::Option<Type>, Type):Type)(func_ret, (Unknown{}: Type)))))), FuncArg[(FuncArg{.func=(func: bit<32>), .name=(_: internment::Intern<string>), .ty=(ty: Type)}: FuncArg)] by (func, ret, scope, name)" .to_string(),
                                                                                          afun: &{fn __f(__v: DDValue) -> Option<(DDValue,DDValue)>
                                                                                          {
                                                                                              let (ref name, ref func, ref scope, ref func_ret, ref ret, ref ty) = unsafe { Value::__Tuple6__internment_Intern____Stringval___Bitval32___Bitval32_ddlog_std_Option__Type_ddlog_std_Ref__Type_Type::from_ddvalue_ref( &__v ) }.0;
                                                                                              Some((Value::__Tuple4____Bitval32_ddlog_std_Ref__Type___Bitval32_internment_Intern____Stringval(((*func).clone(), (*ret).clone(), (*scope).clone(), (*name).clone())).into_ddvalue(), Value::__Tuple6__internment_Intern____Stringval___Bitval32___Bitval32_ddlog_std_Option__Type_ddlog_std_Ref__Type_Type(((*name).clone(), (*func).clone(), (*scope).clone(), (*func_ret).clone(), (*ret).clone(), (*ty).clone())).into_ddvalue()))
                                                                                          }
                                                                                          __f},
                                                                                          next: Box::new(XFormArrangement::Aggregate{
                                                                                                             description: "Function[(Function{.name=(name: internment::Intern<string>), .id=(func: bit<32>), .scope=(scope: bit<32>), .ret=(func_ret: ddlog_std::Option<Type>)}: Function)], ((var ret: ddlog_std::Ref<Type>) = ((ddlog_std::ref_new: function(Type):ddlog_std::Ref<Type>)(((ddlog_std::unwrap_or: function(ddlog_std::Option<Type>, Type):Type)(func_ret, (Unknown{}: Type)))))), FuncArg[(FuncArg{.func=(func: bit<32>), .name=(_: internment::Intern<string>), .ty=(ty: Type)}: FuncArg)], var __group = ty.group_by((func, ret, scope, name))".to_string(),
                                                                                                             ffun: None,
                                                                                                             aggfun: &{fn __f(__key: &DDValue, __group__: &[(&DDValue, Weight)]) -> Option<DDValue>
                                                                                                         {
                                                                                                             let (ref func, ref ret, ref scope, ref name) = unsafe { Value::__Tuple4____Bitval32_ddlog_std_Ref__Type___Bitval32_internment_Intern____Stringval::from_ddvalue_ref( __key ) }.0;
                                                                                                             let ref __group = unsafe{::types::ddlog_std::Group::new_by_ref(((*func).clone(), (*ret).clone(), (*scope).clone(), (*name).clone()), __group__, {fn __f(__v: &DDValue) ->  ::types::Type
                                                                                                                                                                                                                                                             {
                                                                                                                                                                                                                                                                 let (ref name, ref func, ref scope, ref func_ret, ref ret, ref ty) = unsafe { Value::__Tuple6__internment_Intern____Stringval___Bitval32___Bitval32_ddlog_std_Option__Type_ddlog_std_Ref__Type_Type::from_ddvalue_ref( __v ) }.0;
                                                                                                                                                                                                                                                                 (*ty).clone()
                                                                                                                                                                                                                                                             }
                                                                                                                                                                                                                                                             ::std::rc::Rc::new(__f)})};
                                                                                                             let ref args: ::types::ddlog_std::Vec<::types::Type> = match ::types::ddlog_std::group_to_vec(__group) {
                                                                                                                 args => args,
                                                                                                                 _ => return None
                                                                                                             };
                                                                                                             let ref ty: ::types::Type = match (::types::Type::Func{args: (*args).clone(), ret: (*ret).clone()}) {
                                                                                                                 ty => ty,
                                                                                                                 _ => return None
                                                                                                             };
                                                                                                             Some(Value::__Tuple3____Bitval32_internment_Intern____Stringval_Type(((*scope).clone(), (*name).clone(), (*ty).clone())).into_ddvalue())
                                                                                                         }
                                                                                                         __f},
                                                                                                             next: Box::new(Some(XFormCollection::FilterMap{
                                                                                                                                     description: "head of Variable[(Variable{.scope=scope, .name=name, .ty=ty}: Variable)] :- Function[(Function{.name=(name: internment::Intern<string>), .id=(func: bit<32>), .scope=(scope: bit<32>), .ret=(func_ret: ddlog_std::Option<Type>)}: Function)], ((var ret: ddlog_std::Ref<Type>) = ((ddlog_std::ref_new: function(Type):ddlog_std::Ref<Type>)(((ddlog_std::unwrap_or: function(ddlog_std::Option<Type>, Type):Type)(func_ret, (Unknown{}: Type)))))), FuncArg[(FuncArg{.func=(func: bit<32>), .name=(_: internment::Intern<string>), .ty=(ty: Type)}: FuncArg)], var __group = ty.group_by((func, ret, scope, name)), ((var args: ddlog_std::Vec<Type>) = ((ddlog_std::group_to_vec: function(ddlog_std::Group<(bit<32>, ddlog_std::Ref<Type>, bit<32>, internment::Intern<string>),Type>):ddlog_std::Vec<Type>)(__group))), ((var ty: Type) = (Func{.args=args, .ret=ret}: Type))." .to_string(),
                                                                                                                                     fmfun: &{fn __f(__v: DDValue) -> Option<DDValue>
                                                                                                                                     {
                                                                                                                                         let (ref scope, ref name, ref ty) = unsafe { Value::__Tuple3____Bitval32_internment_Intern____Stringval_Type::from_ddvalue_ref( &__v ) }.0;
                                                                                                                                         Some(Value::Variable((::types::Variable{scope: (*scope).clone(), name: (*name).clone(), ty: (*ty).clone()})).into_ddvalue())
                                                                                                                                     }
                                                                                                                                     __f},
                                                                                                                                     next: Box::new(None)
                                                                                                                                 }))
                                                                                                         })
                                                                                      }))
                                                              })
                                           })
                           },
                           /* Variable[(Variable{.scope=scope, .name=name, .ty=ty}: Variable)] :- FuncArg[(FuncArg{.func=(func: bit<32>), .name=(name: internment::Intern<string>), .ty=(ty: Type)}: FuncArg)], Function[(Function{.name=(_: internment::Intern<string>), .id=(func: bit<32>), .scope=(scope: bit<32>), .ret=(_: ddlog_std::Option<Type>)}: Function)]. */
                           Rule::ArrangementRule {
                               description: "Variable[(Variable{.scope=scope, .name=name, .ty=ty}: Variable)] :- FuncArg[(FuncArg{.func=(func: bit<32>), .name=(name: internment::Intern<string>), .ty=(ty: Type)}: FuncArg)], Function[(Function{.name=(_: internment::Intern<string>), .id=(func: bit<32>), .scope=(scope: bit<32>), .ret=(_: ddlog_std::Option<Type>)}: Function)].".to_string(),
                               arr: ( Relations::FuncArg as RelId, 0),
                               xform: XFormArrangement::Join{
                                          description: "FuncArg[(FuncArg{.func=(func: bit<32>), .name=(name: internment::Intern<string>), .ty=(ty: Type)}: FuncArg)], Function[(Function{.name=(_: internment::Intern<string>), .id=(func: bit<32>), .scope=(scope: bit<32>), .ret=(_: ddlog_std::Option<Type>)}: Function)]".to_string(),
                                          ffun: None,
                                          arrangement: (Relations::Function as RelId,0),
                                          jfun: &{fn __f(_: &DDValue ,__v1: &DDValue,__v2: &DDValue) -> Option<DDValue>
                                          {
                                              let (ref func, ref name, ref ty) = match unsafe {  Value::FuncArg::from_ddvalue_ref(__v1) }.0 {
                                                  ::types::FuncArg{func: ref func, name: ref name, ty: ref ty} => ((*func).clone(), (*name).clone(), (*ty).clone()),
                                                  _ => return None
                                              };
                                              let ref scope = match unsafe {  Value::Function::from_ddvalue_ref(__v2) }.0 {
                                                  ::types::Function{name: _, id: _, scope: ref scope, ret: _} => (*scope).clone(),
                                                  _ => return None
                                              };
                                              Some(Value::Variable((::types::Variable{scope: (*scope).clone(), name: (*name).clone(), ty: (*ty).clone()})).into_ddvalue())
                                          }
                                          __f},
                                          next: Box::new(None)
                                      }
                           },
                           /* Variable[(Variable{.scope=scope, .name=name, .ty=ty}: Variable)] :- Variable[(Variable{.scope=(var_scope: bit<32>), .name=(name: internment::Intern<string>), .ty=(ty: Type)}: Variable)], ChildScope[(ChildScope{.parent=(var_scope: bit<32>), .child=(scope: bit<32>)}: ChildScope)]. */
                           Rule::ArrangementRule {
                               description: "Variable[(Variable{.scope=scope, .name=name, .ty=ty}: Variable)] :- Variable[(Variable{.scope=(var_scope: bit<32>), .name=(name: internment::Intern<string>), .ty=(ty: Type)}: Variable)], ChildScope[(ChildScope{.parent=(var_scope: bit<32>), .child=(scope: bit<32>)}: ChildScope)].".to_string(),
                               arr: ( Relations::Variable as RelId, 3),
                               xform: XFormArrangement::Join{
                                          description: "Variable[(Variable{.scope=(var_scope: bit<32>), .name=(name: internment::Intern<string>), .ty=(ty: Type)}: Variable)], ChildScope[(ChildScope{.parent=(var_scope: bit<32>), .child=(scope: bit<32>)}: ChildScope)]".to_string(),
                                          ffun: None,
                                          arrangement: (Relations::ChildScope as RelId,1),
                                          jfun: &{fn __f(_: &DDValue ,__v1: &DDValue,__v2: &DDValue) -> Option<DDValue>
                                          {
                                              let (ref var_scope, ref name, ref ty) = match unsafe {  Value::Variable::from_ddvalue_ref(__v1) }.0 {
                                                  ::types::Variable{scope: ref var_scope, name: ref name, ty: ref ty} => ((*var_scope).clone(), (*name).clone(), (*ty).clone()),
                                                  _ => return None
                                              };
                                              let ref scope = match unsafe {  Value::ChildScope::from_ddvalue_ref(__v2) }.0 {
                                                  ::types::ChildScope{parent: _, child: ref scope} => (*scope).clone(),
                                                  _ => return None
                                              };
                                              Some(Value::Variable((::types::Variable{scope: (*scope).clone(), name: (*name).clone(), ty: (*ty).clone()})).into_ddvalue())
                                          }
                                          __f},
                                          next: Box::new(None)
                                      }
                           }],
                       arrangements: vec![
                           Arrangement::Map{
                              name: r###"(Variable{.scope=(_0: bit<32>), .name=(_1: internment::Intern<string>), .ty=(_: Type)}: Variable) /*join*/"###.to_string(),
                               afun: &{fn __f(__v: DDValue) -> Option<(DDValue,DDValue)>
                               {
                                   let __cloned = __v.clone();
                                   match unsafe { Value::Variable::from_ddvalue(__v) }.0 {
                                       ::types::Variable{scope: ref _0, name: ref _1, ty: _} => Some(Value::__Tuple2____Bitval32_internment_Intern____Stringval(((*_0).clone(), (*_1).clone())).into_ddvalue()),
                                       _ => None
                                   }.map(|x|(x,__cloned))
                               }
                               __f},
                               queryable: false
                           },
                           Arrangement::Map{
                              name: r###"(Variable{.scope=(_0: bit<32>), .name=(_1: internment::Intern<string>), .ty=(Func{.args=(_: ddlog_std::Vec<Type>), .ret=(_: ddlog_std::Ref<Type>)}: Type)}: Variable) /*join*/"###.to_string(),
                               afun: &{fn __f(__v: DDValue) -> Option<(DDValue,DDValue)>
                               {
                                   let __cloned = __v.clone();
                                   match unsafe { Value::Variable::from_ddvalue(__v) }.0 {
                                       ::types::Variable{scope: ref _0, name: ref _1, ty: ::types::Type::Func{args: _, ret: _}} => Some(Value::__Tuple2____Bitval32_internment_Intern____Stringval(((*_0).clone(), (*_1).clone())).into_ddvalue()),
                                       _ => None
                                   }.map(|x|(x,__cloned))
                               }
                               __f},
                               queryable: false
                           },
                           Arrangement::Set{
                               name: r###"(Variable{.scope=(_0: bit<32>), .name=(_1: internment::Intern<string>), .ty=(_: Type)}: Variable) /*antijoin*/"###.to_string(),
                               fmfun: &{fn __f(__v: DDValue) -> Option<DDValue>
                               {
                                   match unsafe { Value::Variable::from_ddvalue(__v) }.0 {
                                       ::types::Variable{scope: ref _0, name: ref _1, ty: _} => Some(Value::__Tuple2____Bitval32_internment_Intern____Stringval(((*_0).clone(), (*_1).clone())).into_ddvalue()),
                                       _ => None
                                   }
                               }
                               __f},
                               distinct: true
                           },
                           Arrangement::Map{
                              name: r###"(Variable{.scope=(_0: bit<32>), .name=(_: internment::Intern<string>), .ty=(_: Type)}: Variable) /*join*/"###.to_string(),
                               afun: &{fn __f(__v: DDValue) -> Option<(DDValue,DDValue)>
                               {
                                   let __cloned = __v.clone();
                                   match unsafe { Value::Variable::from_ddvalue(__v) }.0 {
                                       ::types::Variable{scope: ref _0, name: _, ty: _} => Some(Value::__Bitval32((*_0).clone()).into_ddvalue()),
                                       _ => None
                                   }.map(|x|(x,__cloned))
                               }
                               __f},
                               queryable: false
                           }],
                       change_cb:    Some(sync::Arc::new(sync::Mutex::new(__update_cb.clone())))
                   };
    let UninferedExpr = Relation {
                            name:         "UninferedExpr".to_string(),
                            input:        false,
                            distinct:     true,
                            caching_mode: CachingMode::Set,
                            key_func:     None,
                            id:           Relations::UninferedExpr as RelId,
                            rules:        vec![
                                /* UninferedExpr[(UninferedExpr{.expr=expr}: UninferedExpr)] :- Expression[(Expression{.id=(expr: bit<32>), .func=(_: bit<32>), .kind=(_: ExprKind), .scope=(_: bit<32>)}: Expression)], not ExpressionType[(ExpressionType{.expr=(expr: bit<32>), .ty=(_: Type)}: ExpressionType)]. */
                                Rule::ArrangementRule {
                                    description: "UninferedExpr[(UninferedExpr{.expr=expr}: UninferedExpr)] :- Expression[(Expression{.id=(expr: bit<32>), .func=(_: bit<32>), .kind=(_: ExprKind), .scope=(_: bit<32>)}: Expression)], not ExpressionType[(ExpressionType{.expr=(expr: bit<32>), .ty=(_: Type)}: ExpressionType)].".to_string(),
                                    arr: ( Relations::Expression as RelId, 2),
                                    xform: XFormArrangement::Antijoin {
                                               description: "Expression[(Expression{.id=(expr: bit<32>), .func=(_: bit<32>), .kind=(_: ExprKind), .scope=(_: bit<32>)}: Expression)], not ExpressionType[(ExpressionType{.expr=(expr: bit<32>), .ty=(_: Type)}: ExpressionType)]".to_string(),
                                               ffun: None,
                                               arrangement: (Relations::ExpressionType as RelId,1),
                                               next: Box::new(Some(XFormCollection::FilterMap{
                                                                       description: "head of UninferedExpr[(UninferedExpr{.expr=expr}: UninferedExpr)] :- Expression[(Expression{.id=(expr: bit<32>), .func=(_: bit<32>), .kind=(_: ExprKind), .scope=(_: bit<32>)}: Expression)], not ExpressionType[(ExpressionType{.expr=(expr: bit<32>), .ty=(_: Type)}: ExpressionType)]." .to_string(),
                                                                       fmfun: &{fn __f(__v: DDValue) -> Option<DDValue>
                                                                       {
                                                                           let ref expr = match unsafe {  Value::Expression::from_ddvalue_ref(&__v) }.0 {
                                                                               ::types::Expression{id: ref expr, func: _, kind: _, scope: _} => (*expr).clone(),
                                                                               _ => return None
                                                                           };
                                                                           Some(Value::UninferedExpr((::types::UninferedExpr{expr: (*expr).clone()})).into_ddvalue())
                                                                       }
                                                                       __f},
                                                                       next: Box::new(None)
                                                                   }))
                                           }
                                },
                                /* UninferedExpr[(UninferedExpr{.expr=expr}: UninferedExpr)] :- Expression[(Expression{.id=(expr: bit<32>), .func=(_: bit<32>), .kind=(_: ExprKind), .scope=(_: bit<32>)}: Expression)], ExpressionType[(ExpressionType{.expr=(expr: bit<32>), .ty=(Poison{}: Type)}: ExpressionType)]. */
                                Rule::ArrangementRule {
                                    description: "UninferedExpr[(UninferedExpr{.expr=expr}: UninferedExpr)] :- Expression[(Expression{.id=(expr: bit<32>), .func=(_: bit<32>), .kind=(_: ExprKind), .scope=(_: bit<32>)}: Expression)], ExpressionType[(ExpressionType{.expr=(expr: bit<32>), .ty=(Poison{}: Type)}: ExpressionType)].".to_string(),
                                    arr: ( Relations::Expression as RelId, 2),
                                    xform: XFormArrangement::Semijoin{
                                               description: "Expression[(Expression{.id=(expr: bit<32>), .func=(_: bit<32>), .kind=(_: ExprKind), .scope=(_: bit<32>)}: Expression)], ExpressionType[(ExpressionType{.expr=(expr: bit<32>), .ty=(Poison{}: Type)}: ExpressionType)]".to_string(),
                                               ffun: None,
                                               arrangement: (Relations::ExpressionType as RelId,2),
                                               jfun: &{fn __f(_: &DDValue ,__v1: &DDValue,___v2: &()) -> Option<DDValue>
                                               {
                                                   let ref expr = match unsafe {  Value::Expression::from_ddvalue_ref(__v1) }.0 {
                                                       ::types::Expression{id: ref expr, func: _, kind: _, scope: _} => (*expr).clone(),
                                                       _ => return None
                                                   };
                                                   Some(Value::UninferedExpr((::types::UninferedExpr{expr: (*expr).clone()})).into_ddvalue())
                                               }
                                               __f},
                                               next: Box::new(None)
                                           }
                                }],
                            arrangements: vec![
                                ],
                            change_cb:    Some(sync::Arc::new(sync::Mutex::new(__update_cb.clone())))
                        };
    let OutOfScopeVar = Relation {
                            name:         "OutOfScopeVar".to_string(),
                            input:        false,
                            distinct:     true,
                            caching_mode: CachingMode::Set,
                            key_func:     None,
                            id:           Relations::OutOfScopeVar as RelId,
                            rules:        vec![
                                /* OutOfScopeVar[(OutOfScopeVar{.variable=variable, .used=used}: OutOfScopeVar)] :- Expression[(Expression{.id=(used: bit<32>), .func=(_: bit<32>), .kind=(Var{.v=(variable: internment::Intern<string>)}: ExprKind), .scope=(scope: bit<32>)}: Expression)], not Variable[(Variable{.scope=(scope: bit<32>), .name=(variable: internment::Intern<string>), .ty=(_: Type)}: Variable)]. */
                                Rule::ArrangementRule {
                                    description: "OutOfScopeVar[(OutOfScopeVar{.variable=variable, .used=used}: OutOfScopeVar)] :- Expression[(Expression{.id=(used: bit<32>), .func=(_: bit<32>), .kind=(Var{.v=(variable: internment::Intern<string>)}: ExprKind), .scope=(scope: bit<32>)}: Expression)], not Variable[(Variable{.scope=(scope: bit<32>), .name=(variable: internment::Intern<string>), .ty=(_: Type)}: Variable)].".to_string(),
                                    arr: ( Relations::Expression as RelId, 0),
                                    xform: XFormArrangement::Antijoin {
                                               description: "Expression[(Expression{.id=(used: bit<32>), .func=(_: bit<32>), .kind=(Var{.v=(variable: internment::Intern<string>)}: ExprKind), .scope=(scope: bit<32>)}: Expression)], not Variable[(Variable{.scope=(scope: bit<32>), .name=(variable: internment::Intern<string>), .ty=(_: Type)}: Variable)]".to_string(),
                                               ffun: None,
                                               arrangement: (Relations::Variable as RelId,2),
                                               next: Box::new(Some(XFormCollection::FilterMap{
                                                                       description: "head of OutOfScopeVar[(OutOfScopeVar{.variable=variable, .used=used}: OutOfScopeVar)] :- Expression[(Expression{.id=(used: bit<32>), .func=(_: bit<32>), .kind=(Var{.v=(variable: internment::Intern<string>)}: ExprKind), .scope=(scope: bit<32>)}: Expression)], not Variable[(Variable{.scope=(scope: bit<32>), .name=(variable: internment::Intern<string>), .ty=(_: Type)}: Variable)]." .to_string(),
                                                                       fmfun: &{fn __f(__v: DDValue) -> Option<DDValue>
                                                                       {
                                                                           let (ref used, ref variable, ref scope) = match unsafe {  Value::Expression::from_ddvalue_ref(&__v) }.0 {
                                                                               ::types::Expression{id: ref used, func: _, kind: ::types::ExprKind::Var{v: ref variable}, scope: ref scope} => ((*used).clone(), (*variable).clone(), (*scope).clone()),
                                                                               _ => return None
                                                                           };
                                                                           Some(Value::OutOfScopeVar((::types::OutOfScopeVar{variable: (*variable).clone(), used: (*used).clone()})).into_ddvalue())
                                                                       }
                                                                       __f},
                                                                       next: Box::new(None)
                                                                   }))
                                           }
                                }],
                            arrangements: vec![
                                ],
                            change_cb:    Some(sync::Arc::new(sync::Mutex::new(__update_cb.clone())))
                        };
    let INPUT_VarDecl = Relation {
                            name:         "INPUT_VarDecl".to_string(),
                            input:        false,
                            distinct:     false,
                            caching_mode: CachingMode::Set,
                            key_func:     None,
                            id:           Relations::INPUT_VarDecl as RelId,
                            rules:        vec![
                                /* INPUT_VarDecl[x] :- VarDecl[(x: VarDecl)]. */
                                Rule::CollectionRule {
                                    description: "INPUT_VarDecl[x] :- VarDecl[(x: VarDecl)].".to_string(),
                                    rel: Relations::VarDecl as RelId,
                                    xform: Some(XFormCollection::FilterMap{
                                                    description: "head of INPUT_VarDecl[x] :- VarDecl[(x: VarDecl)]." .to_string(),
                                                    fmfun: &{fn __f(__v: DDValue) -> Option<DDValue>
                                                    {
                                                        let ref x = match unsafe {  Value::VarDecl::from_ddvalue_ref(&__v) }.0 {
                                                            ref x => (*x).clone(),
                                                            _ => return None
                                                        };
                                                        Some(Value::VarDecl((*x).clone()).into_ddvalue())
                                                    }
                                                    __f},
                                                    next: Box::new(None)
                                                })
                                }],
                            arrangements: vec![
                                ],
                            change_cb:    Some(sync::Arc::new(sync::Mutex::new(__update_cb.clone())))
                        };
    let __Null = Relation {
                     name:         "__Null".to_string(),
                     input:        false,
                     distinct:     false,
                     caching_mode: CachingMode::Set,
                     key_func:     None,
                     id:           Relations::__Null as RelId,
                     rules:        vec![
                         ],
                     arrangements: vec![
                         Arrangement::Map{
                            name: r###"_ /*join*/"###.to_string(),
                             afun: &{fn __f(__v: DDValue) -> Option<(DDValue,DDValue)>
                             {
                                 let __cloned = __v.clone();
                                 match unsafe { Value::__Tuple0__::from_ddvalue(__v) }.0 {
                                     _ => Some(Value::__Tuple0__(()).into_ddvalue()),
                                     _ => None
                                 }.map(|x|(x,__cloned))
                             }
                             __f},
                             queryable: true
                         }],
                     change_cb:    None
                 };
    Program {
        nodes: vec![
            ProgNode::Rel{rel: Application},
            ProgNode::Rel{rel: INPUT_Application},
            ProgNode::Rel{rel: ApplicationArg},
            ProgNode::Rel{rel: INPUT_ApplicationArg},
            ProgNode::Rel{rel: Expression},
            ProgNode::Rel{rel: NonexistantFunction},
            ProgNode::Rel{rel: INPUT_Expression},
            ProgNode::Rel{rel: FuncArg},
            ProgNode::Rel{rel: INPUT_FuncArg},
            ProgNode::Rel{rel: Function},
            ProgNode::Rel{rel: INPUT_Function},
            ProgNode::Rel{rel: InputScope},
            ProgNode::SCC{rels: vec![RecursiveRelation{rel: ChildScope, distinct: true}]},
            ProgNode::Rel{rel: INPUT_InputScope},
            ProgNode::Rel{rel: Literal},
            ProgNode::Rel{rel: INPUT_Literal},
            ProgNode::Rel{rel: VarDecl},
            ProgNode::SCC{rels: vec![RecursiveRelation{rel: ExpressionType, distinct: true}, RecursiveRelation{rel: Variable, distinct: true}]},
            ProgNode::Rel{rel: UninferedExpr},
            ProgNode::Rel{rel: OutOfScopeVar},
            ProgNode::Rel{rel: INPUT_VarDecl},
            ProgNode::Rel{rel: __Null}
        ],
        init_data: vec![
        ]
    }
}