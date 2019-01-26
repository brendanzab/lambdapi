use codespan::{ByteIndex, ByteSpan, CodeMap};
use moniker::{assert_term_eq, FreeVar, Var};
use pretty_assertions::assert_eq;

use pikelet_concrete::desugar::{Desugar, DesugarEnv};
use pikelet_concrete::elaborate::{self, Context, TypeError};
use pikelet_concrete::syntax::{concrete, raw};

mod support;

#[test]
fn undefined_name() {
    use pikelet_core::syntax::LevelShift;

    let context = Context::default();

    let x = FreeVar::fresh_named("x");
    let given_expr = raw::RcTerm::from(raw::Term::Var(
        ByteSpan::default(),
        Var::Free(x.clone()),
        LevelShift(0),
    ));

    assert_eq!(
        elaborate::infer_term(&context, &given_expr),
        Err(TypeError::UndefinedName {
            span: ByteSpan::default(),
            free_var: x.clone(),
        }),
    );
}

#[test]
fn import_not_found() {
    let mut codemap = CodeMap::new();
    let context = Context::default();
    let desugar_env = DesugarEnv::new(context.mappings());

    let given_expr = r#"import "does-not-exist" : Record {}"#;

    let raw_term = support::parse_term(&mut codemap, given_expr)
        .desugar(&desugar_env)
        .unwrap();

    match elaborate::infer_term(&context, &raw_term) {
        Err(TypeError::UndefinedImport { .. }) => {},
        Err(err) => panic!("unexpected error: {:?}", err),
        Ok((term, ty)) => panic!("expected error, found {} : {:?}", term, ty),
    }
}

#[test]
fn ty() {
    let mut codemap = CodeMap::new();
    let context = Context::default();

    let expected_ty = r"Type^1";
    let given_expr = r"Type";

    assert_term_eq!(
        support::parse_infer_term(&mut codemap, &context, given_expr).1,
        support::parse_nf_term(&mut codemap, &context, expected_ty),
    );
}

#[test]
fn ty_levels() {
    let mut codemap = CodeMap::new();
    let context = Context::default();

    let expected_ty = r"Type^1";
    let given_expr = r"Type^0 : Type^1 : Type^2 : Type^3"; //... Type^∞       ...+:｡(ﾉ･ω･)ﾉﾞ

    assert_term_eq!(
        support::parse_infer_term(&mut codemap, &context, given_expr).1,
        support::parse_nf_term(&mut codemap, &context, expected_ty),
    );
}

#[test]
fn ann_ty_id() {
    let mut codemap = CodeMap::new();
    let context = Context::default();

    let expected_ty = r"Type -> Type";
    let given_expr = r"(fun a => a) : Type -> Type";

    assert_term_eq!(
        support::parse_infer_term(&mut codemap, &context, given_expr).1,
        support::parse_nf_term(&mut codemap, &context, expected_ty),
    );
}

#[test]
fn ann_arrow_ty_id() {
    let mut codemap = CodeMap::new();
    let context = Context::default();

    let expected_ty = r"(Type -> Type) -> (Type -> Type)";
    let given_expr = r"(fun a => a) : (Type -> Type) -> (Type -> Type)";

    assert_term_eq!(
        support::parse_infer_term(&mut codemap, &context, given_expr).1,
        support::parse_nf_term(&mut codemap, &context, expected_ty),
    );
}

#[test]
fn ann_id_as_ty() {
    let mut codemap = CodeMap::new();
    let context = Context::default();
    let desugar_env = DesugarEnv::new(context.mappings());

    let given_expr = r"(fun a => a) : Type";

    let raw_term = support::parse_term(&mut codemap, given_expr)
        .desugar(&desugar_env)
        .unwrap();

    match elaborate::infer_term(&context, &raw_term) {
        Err(TypeError::UnexpectedFunction { .. }) => {},
        other => panic!("unexpected result: {:#?}", other),
    }
}

#[test]
fn fun_app() {
    let mut codemap = CodeMap::new();
    let context = Context::default();

    let expected_ty = r"Type^1";
    let given_expr = r"(fun (a : Type^1) => a) Type";

    assert_term_eq!(
        support::parse_infer_term(&mut codemap, &context, given_expr).1,
        support::parse_nf_term(&mut codemap, &context, expected_ty),
    );
}

#[test]
fn fun_app_ty() {
    let mut codemap = CodeMap::new();
    let context = Context::default();
    let desugar_env = DesugarEnv::new(context.mappings());

    let given_expr = r"Type Type";

    let raw_term = support::parse_term(&mut codemap, given_expr)
        .desugar(&desugar_env)
        .unwrap();

    assert_eq!(
        elaborate::infer_term(&context, &raw_term),
        Err(TypeError::ArgAppliedToNonFunction {
            fn_span: ByteSpan::new(ByteIndex(1), ByteIndex(5)),
            arg_span: ByteSpan::new(ByteIndex(6), ByteIndex(10)),
            found: Box::new(concrete::Term::Universe(ByteSpan::default(), Some(1))),
        }),
    )
}

#[test]
fn fun_intro() {
    let mut codemap = CodeMap::new();
    let context = Context::default();

    let expected_ty = r"Fun (a : Type) -> Type";
    let given_expr = r"fun (a : Type) => a";

    assert_term_eq!(
        support::parse_infer_term(&mut codemap, &context, given_expr).1,
        support::parse_nf_term(&mut codemap, &context, expected_ty),
    );
}

#[test]
fn fun_ty() {
    let mut codemap = CodeMap::new();
    let context = Context::default();

    let expected_ty = r"Type^1";
    let given_expr = r"Fun (a : Type) -> a";

    assert_term_eq!(
        support::parse_infer_term(&mut codemap, &context, given_expr).1,
        support::parse_nf_term(&mut codemap, &context, expected_ty),
    );
}

#[test]
fn id() {
    let mut codemap = CodeMap::new();
    let context = Context::default();

    let expected_ty = r"Fun (a : Type) -> a -> a";
    let given_expr = r"fun (a : Type) (x : a) => x";

    assert_term_eq!(
        support::parse_infer_term(&mut codemap, &context, given_expr).1,
        support::parse_nf_term(&mut codemap, &context, expected_ty),
    );
}

#[test]
fn id_ann() {
    let mut codemap = CodeMap::new();
    let context = Context::default();

    let expected_ty = r"Fun (a : Type) -> a -> a";
    let given_expr = r"(fun a (x : a) => x) : Fun (A : Type) -> A -> A";

    assert_term_eq!(
        support::parse_infer_term(&mut codemap, &context, given_expr).1,
        support::parse_nf_term(&mut codemap, &context, expected_ty),
    );
}

// Passing `Type` to the polymorphic identity function should yield the type
// identity function
#[test]
fn id_fun_app_ty() {
    let mut codemap = CodeMap::new();
    let context = Context::default();

    let expected_ty = r"Type -> Type";
    let given_expr = r"(fun (a : Type^1) (x : a) => x) Type";

    assert_term_eq!(
        support::parse_infer_term(&mut codemap, &context, given_expr).1,
        support::parse_nf_term(&mut codemap, &context, expected_ty),
    );
}

// Passing `Type` to the `Type` identity function should yield `Type`
#[test]
fn id_fun_app_ty_ty() {
    let mut codemap = CodeMap::new();
    let context = Context::default();

    let expected_ty = r"Type^1";
    let given_expr = r"(fun (a : Type^2) (x : a) => x) (Type^1) Type";

    assert_term_eq!(
        support::parse_infer_term(&mut codemap, &context, given_expr).1,
        support::parse_nf_term(&mut codemap, &context, expected_ty),
    );
}

#[test]
fn id_fun_app_ty_arr_ty() {
    let mut codemap = CodeMap::new();
    let context = Context::default();

    let expected_ty = r"Type^1";
    let given_expr = r"(fun (a : Type^2) (x : a) => x) (Type^1) (Type -> Type)";

    assert_term_eq!(
        support::parse_infer_term(&mut codemap, &context, given_expr).1,
        support::parse_nf_term(&mut codemap, &context, expected_ty),
    );
}

#[test]
fn id_fun_app_arr_fun_ty() {
    let mut codemap = CodeMap::new();
    let context = Context::default();

    let expected_ty = r"Type -> Type";
    let given_expr = r"(fun (a : Type^1) (x : a) => x) (Type -> Type) (fun x => x)";

    assert_term_eq!(
        support::parse_infer_term(&mut codemap, &context, given_expr).1,
        support::parse_nf_term(&mut codemap, &context, expected_ty),
    );
}

#[test]
fn apply() {
    let mut codemap = CodeMap::new();
    let context = Context::default();

    let expected_ty = r"Fun (a b : Type) -> (a -> b) -> a -> b";
    let given_expr = r"fun (a b : Type) (f : a -> b) (x : a) => f x";

    assert_term_eq!(
        support::parse_infer_term(&mut codemap, &context, given_expr).1,
        support::parse_nf_term(&mut codemap, &context, expected_ty),
    );
}

#[test]
fn const_() {
    let mut codemap = CodeMap::new();
    let context = Context::default();

    let expected_ty = r"Fun (a b : Type) -> a -> b -> a";
    let given_expr = r"fun (a b : Type) (x : a) (y : b) => x";

    assert_term_eq!(
        support::parse_infer_term(&mut codemap, &context, given_expr).1,
        support::parse_nf_term(&mut codemap, &context, expected_ty),
    );
}

#[test]
fn const_flipped() {
    let mut codemap = CodeMap::new();
    let context = Context::default();

    let expected_ty = r"Fun (a b : Type) -> a -> b -> b";
    let given_expr = r"fun (a b : Type) (x : a) (y : b) => y";

    assert_term_eq!(
        support::parse_infer_term(&mut codemap, &context, given_expr).1,
        support::parse_nf_term(&mut codemap, &context, expected_ty),
    );
}

#[test]
fn flip() {
    let mut codemap = CodeMap::new();
    let context = Context::default();

    let expected_ty = r"Fun (a b c : Type) -> (a -> b -> c) -> (b -> a -> c)";
    let given_expr = r"fun (a b c : Type) (f : a -> b -> c) (y : b) (x : a) => f x y";

    assert_term_eq!(
        support::parse_infer_term(&mut codemap, &context, given_expr).1,
        support::parse_nf_term(&mut codemap, &context, expected_ty),
    );
}

#[test]
fn compose() {
    let mut codemap = CodeMap::new();
    let context = Context::default();

    let expected_ty = r"Fun (a b c : Type) -> (b -> c) -> (a -> b) -> (a -> c)";
    let given_expr = r"fun (a b c : Type) (f : b -> c) (g : a -> b) (x : a) => f (g x)";

    assert_term_eq!(
        support::parse_infer_term(&mut codemap, &context, given_expr).1,
        support::parse_nf_term(&mut codemap, &context, expected_ty),
    );
}

#[test]
fn let_expr_1() {
    let mut codemap = CodeMap::new();
    let context = Context::default();

    let expected_ty = r"String";
    let given_expr = r#"
        let x = "helloo";
        in
            x
    "#;

    assert_term_eq!(
        support::parse_infer_term(&mut codemap, &context, given_expr).1,
        support::parse_nf_term(&mut codemap, &context, expected_ty),
    );
}

#[test]
fn let_expr_2() {
    let mut codemap = CodeMap::new();
    let context = Context::default();

    let expected_ty = r"String";
    let given_expr = r#"
        let x = "helloo";
            y = x;
        in
            x
    "#;

    assert_term_eq!(
        support::parse_infer_term(&mut codemap, &context, given_expr).1,
        support::parse_nf_term(&mut codemap, &context, expected_ty),
    );
}

#[test]
fn let_shift_universes() {
    let mut codemap = CodeMap::new();
    let context = Context::default();

    let given_expr = r#"
        let
            id : Fun (a : Type) -> a -> a;
            id a x = x;

            test1 = id String "hello";
            test2 = id S32 1;
            test3 = id^1 Type String;
            test4 = id^2 Type String;
            test5 = id^2 Type^1 String;
            test6 = id^2 Type^1 Type;

            id1 : Fun (a : Type^1) -> a -> a = id^1;
            id2 : Fun (a : Type^2) -> a -> a = id^2;
            id11 : Fun (a : Type^2) -> a -> a = id1^1;
            id22 : Fun (a : Type^4) -> a -> a = id2^2;
        in
            record {}
    "#;

    support::parse_infer_term(&mut codemap, &context, given_expr);
}

#[test]
fn let_shift_universes_id_self_application() {
    let mut codemap = CodeMap::new();
    let context = Context::default();

    // Here is a curious example from the Idris docs:
    // http://docs.idris-lang.org/en/v1.3.0/tutorial/miscellany.html#cumulativity
    //
    // ```idris
    // myid : (a : Type) -> a -> a
    // myid _ x = x
    //
    // idid : (a : Type) -> a -> a
    // idid = myid _ myid
    // ```
    //
    // This would cause a cycle in the universe hierarchy in Idris, but is
    // perfectly ok when implemented using explicit universe shifting.

    let given_expr = r#"
        let
            id : Fun (a : Type) -> a -> a;
            id a x = x;

            id-id : Fun (a : Type) -> a -> a;
            id-id = id^1 (Fun (a : Type) -> a -> a) id;
        in
            record {}
    "#;

    support::parse_infer_term(&mut codemap, &context, given_expr);
}

#[test]
fn let_shift_universes_literals() {
    let mut codemap = CodeMap::new();
    let context = Context::default();

    let given_expr = r#"
        let
            id : Fun (a : Type) -> a -> a;
            id a x = x;

            test2 = "hello" : id^1 Type String;
        in
            record {}
    "#;

    support::parse_infer_term(&mut codemap, &context, given_expr);
}

#[test]
fn let_shift_universes_literals_bad() {
    let mut codemap = CodeMap::new();
    let context = Context::default();
    let desugar_env = DesugarEnv::new(context.mappings());

    let given_expr = r#"
        let
            id : Fun (a : Type) -> a -> a;
            id a x = x;

            test2 = "hello" : id^2 Type^1 String^1;
        in
            record {}
    "#;

    let raw_term = support::parse_term(&mut codemap, given_expr)
        .desugar(&desugar_env)
        .unwrap();

    match elaborate::infer_term(&context, &raw_term) {
        Ok(_) => panic!("expected error"),
        Err(TypeError::LiteralMismatch { .. }) => {},
        Err(err) => panic!("unexpected error: {}", err),
    }
}

#[test]
fn let_shift_universes_too_little() {
    let mut codemap = CodeMap::new();
    let context = Context::default();
    let desugar_env = DesugarEnv::new(context.mappings());

    let given_expr = r#"
        let
            id : Fun (a : Type) -> a -> a;
            id a x = x;

            test1 = id^1 Type^1 Type;
        in
            record {}
    "#;

    let raw_term = support::parse_term(&mut codemap, given_expr)
        .desugar(&desugar_env)
        .unwrap();

    match elaborate::infer_term(&context, &raw_term) {
        Ok(_) => panic!("expected error"),
        Err(TypeError::Mismatch { .. }) => {},
        Err(err) => panic!("unexpected error: {}", err),
    }
}

#[test]
fn let_shift_universes_too_much() {
    let mut codemap = CodeMap::new();
    let context = Context::default();

    let given_expr = r#"
        let
            id : Fun (a : Type) -> a -> a;
            id a x = x;

            test1 = id^2 Type String;
        in
            record {}
    "#;

    support::parse_infer_term(&mut codemap, &context, given_expr);
}

#[test]
fn case_expr() {
    let mut codemap = CodeMap::new();
    let context = Context::default();

    let expected_ty = r"String";
    let given_expr = r#"case "helloo" {
        "hi" => "haha";
        "hello" => "byee";
        greeting => (import "prim/string/append") greeting "!!";
    }"#;

    assert_term_eq!(
        support::parse_infer_term(&mut codemap, &context, given_expr).1,
        support::parse_nf_term(&mut codemap, &context, expected_ty),
    );
}

#[test]
fn case_expr_bool() {
    let mut codemap = CodeMap::new();
    let context = Context::default();

    let expected_ty = r"String";
    let given_expr = r#"case true {
        true => "hello";
        false => "hi";
    }"#;

    assert_term_eq!(
        support::parse_infer_term(&mut codemap, &context, given_expr).1,
        support::parse_nf_term(&mut codemap, &context, expected_ty),
    );
}

#[test]
fn case_expr_bool_bad() {
    let mut codemap = CodeMap::new();
    let context = Context::default();
    let desugar_env = DesugarEnv::new(context.mappings());

    let given_expr = r#"case "hello" {
        true => "hello";
        false => "hi";
    }"#;

    let raw_term = support::parse_term(&mut codemap, given_expr)
        .desugar(&desugar_env)
        .unwrap();

    match elaborate::infer_term(&context, &raw_term) {
        Err(TypeError::Mismatch { .. }) => {},
        Err(err) => panic!("unexpected error: {:?}", err),
        Ok((term, ty)) => panic!("expected error, found {} : {:?}", term, ty),
    }
}

#[test]
fn case_expr_wildcard() {
    let mut codemap = CodeMap::new();
    let context = Context::default();

    let expected_ty = r"String";
    let given_expr = r#"case "helloo" {
        test => test;
    }"#;

    assert_term_eq!(
        support::parse_infer_term(&mut codemap, &context, given_expr).1,
        support::parse_nf_term(&mut codemap, &context, expected_ty),
    );
}

#[test]
fn case_expr_empty() {
    let mut codemap = CodeMap::new();
    let context = Context::default();
    let desugar_env = DesugarEnv::new(context.mappings());

    let given_expr = r#"case "helloo" {}"#;

    let raw_term = support::parse_term(&mut codemap, given_expr)
        .desugar(&desugar_env)
        .unwrap();

    match elaborate::infer_term(&context, &raw_term) {
        Err(TypeError::AmbiguousEmptyCase { .. }) => {},
        other => panic!("unexpected result: {:#?}", other),
    }
}

mod church_encodings {
    use super::*;

    #[test]
    fn void() {
        let mut codemap = CodeMap::new();
        let context = Context::default();

        let given_expr = r"
        record {
            void; not;
        } where {
            ||| Logical absurdity
            void : Type^1;
            void = Fun (a : Type) -> a;


            ||| Logical negation
            not : Type -> Type^1;
            not a = a -> void;
        }
    ";

        support::parse_infer_term(&mut codemap, &context, given_expr);
    }

    #[test]
    fn unit() {
        let mut codemap = CodeMap::new();
        let context = Context::default();

        let given_expr = r"
        record {
            unit; unit-intro; unit-elim;
        } where {
            unit : Type^1;
            unit = Fun (a : Type) -> a -> a;

            unit-intro : unit;
            unit-intro a x = x;

            unit-elim : Fun (a : Type) -> unit -> a -> a;
            unit-elim a f x = f a x;
        }
    ";

        support::parse_infer_term(&mut codemap, &context, given_expr);
    }

    #[test]
    fn and() {
        let mut codemap = CodeMap::new();
        let context = Context::default();

        let given_expr = r"
            record {
                and; and-elim-left; and-elim-right;
            } where {
                ||| Logical conjunction (Church encoded)
                |||
                ||| You could also interpret this as a product type
                and : Type -> Type -> Type^1;
                and p q = Fun (c : Type) -> (p -> q -> c) -> c;

                ||| Introduce a logical conjunction between two types
                and-intro : Fun (p q : Type) -> p -> q -> and p q;
                and-intro p q x y c f = f x y;

                and-elim-left : Fun (p q : Type) -> and p q -> p;
                and-elim-left p q (pq : and p q) = pq p (fun x y => x);

                and-elim-right : Fun (p q : Type) -> and p q -> q;
                and-elim-right p q (pq : and p q) = pq q (fun x y => y);
            }
        ";

        support::parse_infer_term(&mut codemap, &context, given_expr);
    }

    #[test]
    fn or() {
        let mut codemap = CodeMap::new();
        let context = Context::default();

        let given_expr = r"
            record {
                or; or-intro-left; or-intro-right;
            } where {
                ||| Logical disjunction (Church encoded)
                |||
                ||| You could also interpret this as a sum type
                or : Type -> Type -> Type^1;
                or p q = Fun (c : Type) -> (p -> c) -> (q -> c) -> c;

                or-intro-left : Fun (p q : Type) -> p -> or p q;
                or-intro-left p q x =
                    fun (c : Type) (on-p : p -> c) (on-q : q -> c) => on-p x;

                or-intro-right : Fun (p q : Type) -> q -> or p q;
                or-intro-right p q y =
                    fun (c : Type) (on-p : p -> c) (on-q : q -> c) => on-q y;
            }
        ";

        support::parse_infer_term(&mut codemap, &context, given_expr);
    }
}

#[test]
fn empty_record_ty() {
    let mut codemap = CodeMap::new();
    let context = Context::default();

    let expected_ty = r"Type";
    let given_expr = r"Record {}";

    assert_term_eq!(
        support::parse_infer_term(&mut codemap, &context, given_expr).1,
        support::parse_nf_term(&mut codemap, &context, expected_ty),
    );
}

#[test]
fn empty_record_intro() {
    let mut codemap = CodeMap::new();
    let context = Context::default();

    let expected_ty = r"Record {}";
    let given_expr = r"record {}";

    assert_term_eq!(
        support::parse_infer_term(&mut codemap, &context, given_expr).1,
        support::parse_nf_term(&mut codemap, &context, expected_ty),
    );
}

#[test]
fn dependent_record_ty() {
    let mut codemap = CodeMap::new();
    let context = Context::default();

    let expected_ty = r"Type^2";
    let given_expr = r"Record { t : Type^1; x : t }";

    assert_term_eq!(
        support::parse_infer_term(&mut codemap, &context, given_expr).1,
        support::parse_nf_term(&mut codemap, &context, expected_ty),
    );
}

#[test]
fn record_intro() {
    let mut codemap = CodeMap::new();
    let context = Context::default();

    let expected_ty = r"Record { t : Type; x : String }";
    let given_expr = r#"record { t = String; x = "Hello" }"#;

    assert_term_eq!(
        support::parse_infer_term(&mut codemap, &context, given_expr).1,
        support::parse_nf_term(&mut codemap, &context, expected_ty),
    );
}

#[test]
fn record_proj() {
    let mut codemap = CodeMap::new();
    let context = Context::default();

    let expected_ty = r"String";
    let given_expr = r#"(record { t = String; x = "hello" } : Record { t : Type; x : String }).x"#;

    assert_term_eq!(
        support::parse_infer_term(&mut codemap, &context, given_expr).1,
        support::parse_nf_term(&mut codemap, &context, expected_ty),
    );
}

#[test]
fn record_proj_missing() {
    let mut codemap = CodeMap::new();
    let context = Context::default();
    let desugar_env = DesugarEnv::new(context.mappings());

    let given_expr = r#"(record { x = "hello" } : Record { x : String }).bloop"#;

    let raw_term = support::parse_term(&mut codemap, given_expr)
        .desugar(&desugar_env)
        .unwrap();

    match elaborate::infer_term(&context, &raw_term) {
        Err(TypeError::NoFieldInType { .. }) => {},
        x => panic!("expected a field lookup error, found {:?}", x),
    }
}

#[test]
fn record_proj_weird1() {
    let mut codemap = CodeMap::new();
    let context = Context::default();

    let expected_ty = r"Type^1";
    let given_expr = r"Record {
        data : Record {
            t : Type;
            x : t;
        };

        f : data.t -> Type;
        test : f data.x;
    }";

    assert_term_eq!(
        support::parse_infer_term(&mut codemap, &context, given_expr).1,
        support::parse_nf_term(&mut codemap, &context, expected_ty),
    );
}

#[test]
fn record_proj_weird2() {
    let mut codemap = CodeMap::new();
    let context = Context::default();

    let expected_ty = r"Type^1";
    let given_expr = r"Record {
        Array : U16 -> Type -> Type;
        t : Record { n : U16; x : Array n S8; y : Array n S8 };
        inner-prod : Fun (len : U16) -> Array len S8 -> Array len S8 -> S32;

        test1 : S32 -> Type;
        test2 : test1 (inner-prod t.n t.x t.y);
    }";

    assert_term_eq!(
        support::parse_infer_term(&mut codemap, &context, given_expr).1,
        support::parse_nf_term(&mut codemap, &context, expected_ty),
    );
}

#[test]
fn record_proj_shift() {
    let mut codemap = CodeMap::new();
    let context = Context::default();

    let expected_ty = r"Fun (a : Type^1) -> a -> a";
    let given_expr = r"record {
        id = fun (a : Type) (x : a) => x;
    }.id^1";

    assert_term_eq!(
        support::parse_infer_term(&mut codemap, &context, given_expr).1,
        support::parse_nf_term(&mut codemap, &context, expected_ty),
    );
}

#[test]
fn array_intro_ambiguous() {
    let mut codemap = CodeMap::new();
    let context = Context::default();
    let desugar_env = DesugarEnv::new(context.mappings());

    let given_expr = r#"[1; 2 : S32]"#;

    let raw_term = support::parse_term(&mut codemap, given_expr)
        .desugar(&desugar_env)
        .unwrap();

    match elaborate::infer_term(&context, &raw_term) {
        Err(TypeError::AmbiguousArrayLiteral { .. }) => {},
        Err(err) => panic!("unexpected error: {:?}", err),
        Ok((term, ty)) => panic!("expected error, found {} : {:?}", term, ty),
    }
}
