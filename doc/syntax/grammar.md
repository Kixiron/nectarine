# Formal Grammar

Grammar is written in an [EBNF]-like format with minor modifications  

## Items

An item is a top-level definition of some sort, they're reserved for canonical declarations and may only appear at the file
and module levels

```ebnf
Item ::= FunctionDef | Usage | Module

FunctionDef ::= "fn" Ident (Pattern ":" Type)* ("->" Type)? "=" Expression+

Usage ::= "use" Path

Module ::= "module" Ident "=" Item+
```

## Expressions

A expression is any value-returning thing, they make up the bulk of the language

```ebnf
Expression ::= AscribedExpression
               | BinaryOp
               | Comparison
               | Contract
               | Assignment
               | Application
               | Parentheses
               | Match
               | Return
               | Negated
               | Literal
               | Path

AscribedExpression ::= Expression ":" Type

BinaryOp ::= Expression BinaryOperand Expression

BinaryOperand ::= "+" | "-" | "*" | "/"

Comparison ::= Expression Comparator Expression

Comparator ::= "==" | "!=" | ">" | "<" | ">=" | "<="

Contract ::= "ensure" Expression

Assignment ::= "let" Pattern ":=" Expression

Application ::= Ident Expression+

Parentheses ::= "(" Expression ")"

Match ::= "match" Expression "with" ("|" Pattern "->" Expression)+

Return ::= "return" Expression

Negated ::= "not" Expression

Literal ::= String | Int | Bool

String ::= "\"" [^"\""] "\""

Int ::= [0-9][0-9_]*

Bool ::= "True" | "False"
```

## Types

Types are fairly complex since they involve refinements, which involve expressions

```ebnf
Type ::= Path | Tuple | Refined | Generic

Tuple ::= "(" TupleItems? ")"

TupleItems ::= Type | Type "," TupleItems

Refined ::= "{" Ident ":" Type "=>" Expression "}"

Generic ::= "'" Ident
```

## Misc

The in-between bits of the language

```ebnf
Ident ::= [a-zA-Z_][a-zA-Z0-9_]*

Path ::= Ident ("." Ident)*

Pattern ::= Literal | Path
```

[EBNF]: (https://en.wikipedia.org/wiki/Extended_Backus%E2%80%93Naur_form)
