use codespan::{ByteSpan, CodeMap, FileName};
use codespan_reporting::termcolor::{ColorChoice, StandardStream};
use goldenfile::Mint;
use moniker::{assert_term_eq, Binder, Embed, FreeVar, Scope, Var};
use pretty_assertions::assert_eq;
use std::io::Write;

use pikelet_concrete::desugar::{Desugar, DesugarEnv, DesugarError};
use pikelet_concrete::parse;
use pikelet_concrete::syntax::raw::{RcTerm, Term};
use pikelet_concrete::syntax::{concrete, raw};
use pikelet_core::syntax::{Level, LevelShift};

fn golden(filename: &str, literal: &str) {
    let path = "tests/goldenfiles";

    let mut mint = Mint::new(path);
    let mut file = mint.new_goldenfile(filename).unwrap();
    let env = DesugarEnv::new(im::HashMap::new());

    let term = parse_desugar_term(&env, literal);

    write!(file, "{:#?}", term).unwrap();
}

fn parse_term(codemap: &mut CodeMap, src: &str) -> concrete::Term {
    let filemap = codemap.add_filemap(FileName::virtual_("test"), src.into());
    let (concrete_term, _import_paths, errors) = parse::term(&filemap);

    if !errors.is_empty() {
        let writer = StandardStream::stdout(ColorChoice::Always);
        for error in errors {
            codespan_reporting::emit(&mut writer.lock(), &codemap, &error.to_diagnostic()).unwrap();
        }
        panic!("parse error!")
    }

    concrete_term
}

fn parse_desugar_term(env: &DesugarEnv, src: &str) -> raw::RcTerm {
    let mut codemap = CodeMap::new();

    match parse_term(&mut codemap, src).desugar(env) {
        Ok(raw_term) => raw_term,
        Err(error) => {
            let writer = StandardStream::stdout(ColorChoice::Always);
            codespan_reporting::emit(&mut writer.lock(), &codemap, &error.to_diagnostic()).unwrap();
            panic!("type error!");
        },
    }
}

fn var(x: &FreeVar<String>) -> RcTerm {
    RcTerm::from(Term::Var(
        ByteSpan::default(),
        Var::Free(x.clone()),
        LevelShift(0),
    ))
}

fn u0() -> RcTerm {
    RcTerm::from(Term::Universe(ByteSpan::default(), Level(0)))
}

#[test]
fn free_var() {
    let env = DesugarEnv::new(im::HashMap::new());

    match *parse_desugar_term(&env, r"or-elim").inner {
        raw::Term::Var(_, Var::Free(ref free_var), LevelShift(0)) => {
            assert_eq!(free_var.pretty_name, Some("or-elim".to_owned()));
        },
        ref term => panic!("unexpected term: {}", term),
    }
}

#[test]
fn ty() {
    golden("ty", r"Type");
}

#[test]
fn ty_level() {
    golden("ty_level", r"Type^2");
}

#[test]
fn ann() {
    golden("ann", r"Type : Type");
}

#[test]
fn ann_ann_left() {
    golden("ann_ann_left", r"Type : Type : Type");
}

#[test]
fn ann_ann_right() {
    golden("ann_ann_right", r"Type : (Type : Type)");
}

#[test]
fn ann_ann_ann() {
    golden("ann_ann_ann", r"(Type : Type) : (Type : Type)");
}

#[test]
fn fun_intro_ann() {
    let env = DesugarEnv::new(im::HashMap::new());

    let x = FreeVar::fresh_named("x");

    assert_term_eq!(
        parse_desugar_term(&env, r"fun (x : Type -> Type) => x"),
        RcTerm::from(Term::FunIntro(
            ByteSpan::default(),
            Scope::new(
                (
                    Binder(x.clone()),
                    Embed(RcTerm::from(Term::FunType(
                        ByteSpan::default(),
                        Scope::new((Binder(FreeVar::fresh_unnamed()), Embed(u0())), u0()),
                    ))),
                ),
                var(&x),
            ),
        )),
    );
}

#[test]
fn fun_intro() {
    let env = DesugarEnv::new(im::HashMap::new());

    let x = FreeVar::fresh_named("x");
    let y = FreeVar::fresh_named("y");
    let hole = || RcTerm::from(Term::Hole(ByteSpan::default()));

    assert_term_eq!(
        parse_desugar_term(&env, r"fun (x : fun y => y) => x"),
        RcTerm::from(Term::FunIntro(
            ByteSpan::default(),
            Scope::new(
                (
                    Binder(x.clone()),
                    Embed(RcTerm::from(Term::FunIntro(
                        ByteSpan::default(),
                        Scope::new((Binder(y.clone()), Embed(hole())), var(&y)),
                    )))
                ),
                var(&x),
            ),
        )),
    );
}

#[test]
fn fun_intro2_ann() {
    let env = DesugarEnv::new(im::HashMap::new());

    let x = FreeVar::fresh_named("x");
    let y = FreeVar::fresh_named("y");

    assert_term_eq!(
        parse_desugar_term(&env, r"fun (x y : Type) => x"),
        RcTerm::from(Term::FunIntro(
            ByteSpan::default(),
            Scope::new(
                (Binder(x.clone()), Embed(u0())),
                RcTerm::from(Term::FunIntro(
                    ByteSpan::default(),
                    Scope::new((Binder(y.clone()), Embed(u0())), var(&x)),
                )),
            ),
        )),
    );
}

#[test]
fn arrow() {
    let env = DesugarEnv::new(im::HashMap::new());

    assert_term_eq!(
        parse_desugar_term(&env, r"Type -> Type"),
        RcTerm::from(Term::FunType(
            ByteSpan::default(),
            Scope::new((Binder(FreeVar::fresh_unnamed()), Embed(u0())), u0()),
        )),
    );
}

#[test]
fn fun_ty() {
    let env = DesugarEnv::new(im::HashMap::new());

    let x = FreeVar::fresh_named("x");

    assert_term_eq!(
        parse_desugar_term(&env, r"Fun (x : Type -> Type) -> x"),
        RcTerm::from(Term::FunType(
            ByteSpan::default(),
            Scope::new(
                (
                    Binder(x.clone()),
                    Embed(RcTerm::from(Term::FunType(
                        ByteSpan::default(),
                        Scope::new((Binder(FreeVar::fresh_unnamed()), Embed(u0())), u0()),
                    ))),
                ),
                var(&x),
            ),
        )),
    );
}

#[test]
fn fun_ty2() {
    let env = DesugarEnv::new(im::HashMap::new());

    let x = FreeVar::fresh_named("x");
    let y = FreeVar::fresh_named("y");

    assert_term_eq!(
        parse_desugar_term(&env, r"Fun (x y : Type) -> x"),
        RcTerm::from(Term::FunType(
            ByteSpan::default(),
            Scope::new(
                (Binder(x.clone()), Embed(u0())),
                RcTerm::from(Term::FunType(
                    ByteSpan::default(),
                    Scope::new((Binder(y.clone()), Embed(u0())), var(&x)),
                )),
            ),
        )),
    );
}

#[test]
fn fun_ty_arrow() {
    let env = DesugarEnv::new(im::HashMap::new());

    let x = FreeVar::fresh_named("x");

    assert_term_eq!(
        parse_desugar_term(&env, r"Fun (x : Type) -> x -> x"),
        RcTerm::from(Term::FunType(
            ByteSpan::default(),
            Scope::new(
                (Binder(x.clone()), Embed(u0())),
                RcTerm::from(Term::FunType(
                    ByteSpan::default(),
                    Scope::new((Binder(FreeVar::fresh_unnamed()), Embed(var(&x))), var(&x)),
                )),
            ),
        )),
    );
}

#[test]
fn fun_intro_fun_app() {
    let env = DesugarEnv::new(im::HashMap::new());

    let x = FreeVar::fresh_named("x");
    let y = FreeVar::fresh_named("y");

    assert_term_eq!(
        parse_desugar_term(&env, r"fun (x : Type -> Type) (y : Type) => x y"),
        RcTerm::from(Term::FunIntro(
            ByteSpan::default(),
            Scope::new(
                (
                    Binder(x.clone()),
                    Embed(RcTerm::from(Term::FunType(
                        ByteSpan::default(),
                        Scope::new((Binder(FreeVar::fresh_unnamed()), Embed(u0())), u0()),
                    ))),
                ),
                RcTerm::from(Term::FunIntro(
                    ByteSpan::default(),
                    Scope::new(
                        (Binder(y.clone()), Embed(u0())),
                        RcTerm::from(Term::FunApp(var(&x), var(&y))),
                    ),
                )),
            ),
        )),
    );
}

#[test]
fn id() {
    let env = DesugarEnv::new(im::HashMap::new());

    let x = FreeVar::fresh_named("x");
    let a = FreeVar::fresh_named("a");

    assert_term_eq!(
        parse_desugar_term(&env, r"fun (a : Type) (x : a) => x"),
        RcTerm::from(Term::FunIntro(
            ByteSpan::default(),
            Scope::new(
                (Binder(a.clone()), Embed(u0())),
                RcTerm::from(Term::FunIntro(
                    ByteSpan::default(),
                    Scope::new((Binder(x.clone()), Embed(var(&a))), var(&x)),
                )),
            ),
        )),
    );
}

#[test]
fn id_ty() {
    let env = DesugarEnv::new(im::HashMap::new());

    let a = FreeVar::fresh_named("a");

    assert_term_eq!(
        parse_desugar_term(&env, r"Fun (a : Type) -> a -> a"),
        RcTerm::from(Term::FunType(
            ByteSpan::default(),
            Scope::new(
                (Binder(a.clone()), Embed(u0())),
                RcTerm::from(Term::FunType(
                    ByteSpan::default(),
                    Scope::new((Binder(FreeVar::fresh_unnamed()), Embed(var(&a))), var(&a)),
                )),
            ),
        )),
    );
}

#[test]
fn let_forward_declarations() {
    let mut codemap = CodeMap::new();
    let desugar_env = DesugarEnv::new(im::HashMap::new());

    let src = "
        let
            foo : Type;
            bar : Type;
            bar = Record {};
            foo = Record {};
        in
            record {}
    ";

    if let Err(err) = parse_term(&mut codemap, src).desugar(&desugar_env) {
        let writer = StandardStream::stdout(ColorChoice::Always);
        codespan_reporting::emit(&mut writer.lock(), &codemap, &err.to_diagnostic()).unwrap();
        panic!("type error!")
    }
}

// #[test]
// fn let_forward_declarations_forward_ref() {
//     let mut codemap = CodeMap::new();
//     let desugar_env = DesugarEnv::new(im::HashMap::new());

//     let src = "
//         let
//             foo : Type;
//             bar : Type;
//             bar = foo;
//             foo = Record {};
//         in
//             record {}
//     ";

//     match parse_term(&mut codemap, src).desugar(&desugar_env) {
//         Ok(_) => panic!("expected error"),
//         Err(DesugarError::UndefinedName { .. }) => {},
//         Err(err) => panic!("unexpected error: {}", err),
//     }
// }

#[test]
fn let_declaration_after_definition() {
    let mut codemap = CodeMap::new();
    let desugar_env = DesugarEnv::new(im::HashMap::new());

    let src = "
        let
            foo = Record {};
            foo : Type;
        in
            record {}
    ";

    match parse_term(&mut codemap, src).desugar(&desugar_env) {
        Ok(_) => panic!("expected error"),
        Err(DesugarError::DeclarationFollowedDefinition { .. }) => {},
        Err(err) => panic!("unexpected error: {}", err),
    }
}

#[test]
fn let_duplicate_declarations() {
    let mut codemap = CodeMap::new();
    let desugar_env = DesugarEnv::new(im::HashMap::new());

    let src = "
        let
            foo : Type;
            foo : Type;
        in
            record {}
    ";

    match parse_term(&mut codemap, src).desugar(&desugar_env) {
        Ok(_) => panic!("expected error"),
        Err(DesugarError::DuplicateDeclarations { .. }) => {},
        Err(err) => panic!("unexpected error: {}", err),
    }
}

#[test]
fn let_duplicate_definitions() {
    let mut codemap = CodeMap::new();
    let desugar_env = DesugarEnv::new(im::HashMap::new());

    let src = "
        let
            foo = Type;
            foo = Type;
        in
            record {}
    ";

    match parse_term(&mut codemap, src).desugar(&desugar_env) {
        Ok(_) => panic!("expected error"),
        Err(DesugarError::DuplicateDefinitions { .. }) => {},
        Err(err) => panic!("unexpected error: {}", err),
    }
}

mod sugar {
    use super::*;

    #[test]
    fn fun_intro_params() {
        let env = DesugarEnv::new(im::HashMap::new());

        assert_term_eq!(
            parse_desugar_term(&env, r"fun x (y : Type) z => x"),
            parse_desugar_term(&env, r"fun x => fun (y : Type) => fun z => x"),
        );
    }

    #[test]
    fn fun_intro_params_multi() {
        let env = DesugarEnv::new(im::HashMap::new());

        assert_term_eq!(
            parse_desugar_term(&env, r"fun (x : Type) (y : Type) z => x"),
            parse_desugar_term(&env, r"fun (x y : Type) z => x"),
        );
    }

    #[test]
    fn fun_ty_params() {
        let env = DesugarEnv::new(im::HashMap::new());

        assert_term_eq!(
            parse_desugar_term(&env, r"Fun (a : Type) -> Fun (x y z : a) -> x"),
            parse_desugar_term(
                &env,
                r"Fun (a : Type) -> Fun (x : a) -> Fun (y : a) -> Fun (z : a) -> x"
            ),
        );
    }

    #[test]
    fn fun_ty_params_multi() {
        let env = DesugarEnv::new(im::HashMap::new());

        assert_term_eq!(
            parse_desugar_term(&env, r"Fun (a : Type) (x y z : a) (w : x) -> x"),
            parse_desugar_term(
                &env,
                r"Fun (a : Type) -> Fun (x : a) -> Fun (y : a) -> Fun (z : a) -> Fun (w : x) -> x"
            ),
        );
    }

    #[test]
    fn arrow() {
        let env = DesugarEnv::new(im::HashMap::new());

        assert_term_eq!(
            parse_desugar_term(&env, r"Fun (a : Type) -> a -> a"),
            parse_desugar_term(&env, r"Fun (a : Type) -> Fun (x : a) -> a"),
        )
    }

    #[test]
    fn if_then_else() {
        let env = DesugarEnv::new(im::hashmap! {
            "true".to_owned() => FreeVar::fresh_named("true"),
            "false".to_owned() => FreeVar::fresh_named("false"),
        });

        assert_term_eq!(
            parse_desugar_term(&env, r#"if true then "true" else "false""#),
            parse_desugar_term(&env, r#"case true { true => "true"; false => "false" }"#),
        )
    }

    #[test]
    fn record_field_puns() {
        let env = DesugarEnv::new(im::hashmap! {
            "x".to_owned() => FreeVar::fresh_named("x"),
            "y".to_owned() => FreeVar::fresh_named("y"),
        });

        assert_term_eq!(
            parse_desugar_term(&env, r#"record { x; y }"#),
            parse_desugar_term(&env, r#"record { x = x; y = y }"#),
        )
    }
}
