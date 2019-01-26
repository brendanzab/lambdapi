use codespan::CodeMap;
use moniker::{assert_term_eq, Binder, Embed, FreeVar, Scope, Var};
use pretty_assertions::assert_eq;

use pikelet_concrete::elaborate::Context;
use pikelet_core::syntax::core::{RcTerm, Term};
use pikelet_core::syntax::domain::{Neutral, RcNeutral, RcValue, Value};

mod support;

#[test]
fn var() {
    let context = Context::default();

    let x = FreeVar::fresh_named("x");
    let var = RcTerm::from(Term::var(Var::Free(x.clone()), 0));

    assert_eq!(
        pikelet_core::nbe::nf_term(&context, &var).unwrap(),
        RcValue::from(Value::var(Var::Free(x), 0)),
    );
}

#[test]
fn ty() {
    let mut codemap = CodeMap::new();
    let context = Context::default();

    assert_eq!(
        support::parse_nf_term(&mut codemap, &context, r"Type"),
        RcValue::from(Value::universe(0)),
    );
}

#[test]
fn fun_intro() {
    let mut codemap = CodeMap::new();
    let context = Context::default();

    let x = FreeVar::fresh_named("x");

    assert_term_eq!(
        support::parse_nf_term(&mut codemap, &context, r"fun (x : Type) => x"),
        RcValue::from(Value::FunIntro(Scope::new(
            (Binder(x.clone()), Embed(RcValue::from(Value::universe(0)))),
            RcValue::from(Value::var(Var::Free(x), 0)),
        ))),
    );
}

#[test]
fn fun_ty() {
    let mut codemap = CodeMap::new();
    let context = Context::default();

    let x = FreeVar::fresh_named("x");

    assert_term_eq!(
        support::parse_nf_term(&mut codemap, &context, r"Fun (x : Type) -> x"),
        RcValue::from(Value::FunType(Scope::new(
            (Binder(x.clone()), Embed(RcValue::from(Value::universe(0)))),
            RcValue::from(Value::var(Var::Free(x), 0)),
        ))),
    );
}

#[test]
fn fun_intro_fun_app() {
    let mut codemap = CodeMap::new();
    let context = Context::default();

    let given_expr = r"fun (x : Type -> Type) (y : Type) => x y";

    let x = FreeVar::fresh_named("x");
    let y = FreeVar::fresh_named("y");
    let ty_arr = RcValue::from(Value::FunType(Scope::new(
        (
            Binder(FreeVar::fresh_unnamed()),
            Embed(RcValue::from(Value::universe(0))),
        ),
        RcValue::from(Value::universe(0)),
    )));

    assert_term_eq!(
        support::parse_nf_term(&mut codemap, &context, given_expr,),
        RcValue::from(Value::FunIntro(Scope::new(
            (Binder(x.clone()), Embed(ty_arr)),
            RcValue::from(Value::FunIntro(Scope::new(
                (Binder(y.clone()), Embed(RcValue::from(Value::universe(0)))),
                RcValue::from(Value::Neutral(
                    RcNeutral::from(Neutral::var(Var::Free(x), 0)),
                    vec![RcValue::from(Value::var(Var::Free(y), 0))],
                )),
            ))),
        ))),
    );
}

#[test]
fn fun_ty_fun_app() {
    let mut codemap = CodeMap::new();
    let context = Context::default();

    let given_expr = r"Fun (x : Type -> Type) (y : Type) -> x y";

    let x = FreeVar::fresh_named("x");
    let y = FreeVar::fresh_named("y");
    let ty_arr = RcValue::from(Value::FunType(Scope::new(
        (
            Binder(FreeVar::fresh_unnamed()),
            Embed(RcValue::from(Value::universe(0))),
        ),
        RcValue::from(Value::universe(0)),
    )));

    assert_term_eq!(
        support::parse_nf_term(&mut codemap, &context, given_expr),
        RcValue::from(Value::FunType(Scope::new(
            (Binder(x.clone()), Embed(ty_arr)),
            RcValue::from(Value::FunType(Scope::new(
                (Binder(y.clone()), Embed(RcValue::from(Value::universe(0)))),
                RcValue::from(Value::Neutral(
                    RcNeutral::from(Neutral::var(Var::Free(x), 0)),
                    vec![RcValue::from(Value::var(Var::Free(y), 0))],
                )),
            ))),
        ))),
    );
}

// Passing `Type` to the polymorphic identity function should yield the type
// identity function
#[test]
fn id_fun_app_ty() {
    let mut codemap = CodeMap::new();
    let context = Context::default();

    let given_expr = r"(fun (a : Type^1) (x : a) => x) Type";
    let expected_expr = r"fun (x : Type) => x";

    assert_term_eq!(
        support::parse_nf_term(&mut codemap, &context, given_expr),
        support::parse_nf_term(&mut codemap, &context, expected_expr),
    );
}

// Passing `Type` to the `Type` identity function should yield `Type`
#[test]
fn id_fun_app_ty_ty() {
    let mut codemap = CodeMap::new();
    let context = Context::default();

    let given_expr = r"(fun (a : Type^2) (x : a) => x) (Type^1) Type";
    let expected_expr = r"Type";

    assert_term_eq!(
        support::parse_nf_term(&mut codemap, &context, given_expr),
        support::parse_nf_term(&mut codemap, &context, expected_expr),
    );
}

// Passing `Type -> Type` to the `Type` identity function should yield
// `Type -> Type`
#[test]
fn id_fun_app_ty_arr_ty() {
    let mut codemap = CodeMap::new();
    let context = Context::default();

    let given_expr = r"(fun (a : Type^2) (x : a) => x) (Type^1) (Type -> Type)";
    let expected_expr = r"Type -> Type";

    assert_term_eq!(
        support::parse_nf_term(&mut codemap, &context, given_expr),
        support::parse_nf_term(&mut codemap, &context, expected_expr),
    );
}

// Passing the id function to itself should yield the id function
#[test]
fn id_fun_app_id() {
    let mut codemap = CodeMap::new();
    let context = Context::default();

    let given_expr = r"
            (fun (a : Type^1) (x : a) => x)
                (Fun (a : Type) -> a -> a)
                (fun (a : Type) (x : a) => x)
        ";
    let expected_expr = r"fun (a : Type) (x : a) => x";

    assert_term_eq!(
        support::parse_nf_term(&mut codemap, &context, given_expr),
        support::parse_nf_term(&mut codemap, &context, expected_expr),
    );
}

// Passing the id function to the 'const' combinator should yield a
// function that always returns the id function
#[test]
fn const_fun_app_id_ty() {
    let mut codemap = CodeMap::new();
    let context = Context::default();

    let given_expr = r"
        (fun (a : Type^1) (b : Type^2) (x : a) (y : b) => x)
            (Fun (a : Type) -> a -> a)
            (Type^1)
            (fun (a : Type) (x : a) => x)
            Type
    ";
    let expected_expr = r"fun (a : Type) (x : a) => x";

    assert_term_eq!(
        support::parse_nf_term(&mut codemap, &context, given_expr),
        support::parse_nf_term(&mut codemap, &context, expected_expr),
    );
}

#[test]
fn horrifying_fun_app_1() {
    let mut codemap = CodeMap::new();
    let context = Context::default();

    let given_expr = r"
        (fun (t : Type) (f : Fun (a : Type) -> Type) => f t) String (fun (a : Type) => a)
    ";
    let expected_expr = r"String";

    assert_term_eq!(
        support::parse_nf_term(&mut codemap, &context, given_expr),
        support::parse_nf_term(&mut codemap, &context, expected_expr),
    );
}

#[test]
fn horrifying_fun_app_2() {
    let mut codemap = CodeMap::new();
    let context = Context::default();

    let given_expr = r#"(fun (t: String) (f: String -> String) => f t) "hello""#;
    let expected_expr = r#"fun (f : String -> String) => f "hello""#;

    assert_term_eq!(
        support::parse_nf_term(&mut codemap, &context, given_expr),
        support::parse_nf_term(&mut codemap, &context, expected_expr),
    );
}

#[test]
fn let_expr_1() {
    let mut codemap = CodeMap::new();
    let context = Context::default();

    let given_expr = r#"
        let x = "helloo";
        in
            x
    "#;
    let expected_expr = r#"
        "helloo"
    "#;

    assert_term_eq!(
        support::parse_nf_term(&mut codemap, &context, given_expr),
        support::parse_nf_term(&mut codemap, &context, expected_expr),
    );
}

#[test]
fn let_expr_2() {
    let mut codemap = CodeMap::new();
    let context = Context::default();

    let given_expr = r#"
        let x = "helloo";
            y = x;
        in
            x
    "#;
    let expected_expr = r#"
        "helloo"
    "#;

    assert_term_eq!(
        support::parse_nf_term(&mut codemap, &context, given_expr),
        support::parse_nf_term(&mut codemap, &context, expected_expr),
    );
}

#[test]
fn if_true() {
    let mut codemap = CodeMap::new();
    let context = Context::default();

    let given_expr = r#"
        if true then "true" else "false"
    "#;
    let expected_expr = r#"
        "true"
    "#;

    assert_term_eq!(
        support::parse_nf_term(&mut codemap, &context, given_expr),
        support::parse_nf_term(&mut codemap, &context, expected_expr),
    );
}

#[test]
fn if_false() {
    let mut codemap = CodeMap::new();
    let context = Context::default();

    let given_expr = r#"
        if false then "true" else "false"
    "#;
    let expected_expr = r#"
        "false"
    "#;

    assert_term_eq!(
        support::parse_nf_term(&mut codemap, &context, given_expr),
        support::parse_nf_term(&mut codemap, &context, expected_expr),
    );
}

#[test]
fn if_eval_cond() {
    let mut codemap = CodeMap::new();
    let context = Context::default();

    let given_expr = r#"
        let is-hi (greeting : String) = case greeting {
                "hi" => true;
                _ => false;
            };
        in
            record {
                test-hi = if is-hi "hi" then "true" else "false";
                test-bye = if is-hi "bye" then "true" else "false";
            }
    "#;
    let expected_expr = r#"
        record {
            test-hi = "true";
            test-bye = "false";
        }
    "#;

    assert_term_eq!(
        support::parse_nf_term(&mut codemap, &context, given_expr),
        support::parse_nf_term(&mut codemap, &context, expected_expr),
    );
}

#[test]
fn case_expr_bool() {
    let mut codemap = CodeMap::new();
    let context = Context::default();

    let given_expr = r#"
        record {
            test-true = case true {
                true => "true";
                false => "false";
            };
            test-false = case false {
                true => "true";
                false => "false";
            };
        }
    "#;
    let expected_expr = r#"
        record {
            test-true = "true";
            test-false = "false";
        }
    "#;

    assert_term_eq!(
        support::parse_nf_term(&mut codemap, &context, given_expr),
        support::parse_nf_term(&mut codemap, &context, expected_expr),
    );
}

#[test]
fn record_ty_shadow() {
    let mut codemap = CodeMap::new();
    let context = Context::default();

    let given_expr = r"(fun (t : Type) => Record { String : Type; x : t; y : String }) String";
    let expected_expr = r#"Record { String as String1 : Type; x : String; y : String1 }"#;

    assert_term_eq!(
        support::parse_nf_term(&mut codemap, &context, given_expr),
        support::parse_nf_term(&mut codemap, &context, expected_expr),
    );
}
