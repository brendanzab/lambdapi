//! Pretty printing for the core syntax

use nameless::{Name, Var};
use pretty::Doc;

use syntax::core::{Constant, Context, Label, Level, Neutral, RawConstant, RawDefinition,
                   RawModule, RawTerm, Term, Value};

use super::{parens_if, Options, Prec, StaticDoc, ToDoc};

fn pretty_ann<E: ToDoc, T: ToDoc>(options: Options, expr: &E, ty: &T) -> StaticDoc {
    parens_if(
        Prec::ANN < options.prec,
        Doc::group(
            expr.to_doc(options.with_prec(Prec::LAM))
                .append(Doc::space())
                .append(Doc::text(":")),
        ).append(Doc::group(
            Doc::space()
                .append(ty.to_doc(options.with_prec(Prec::ANN)))
                .nest(options.indent_width as usize),
        )),
    )
}

fn pretty_universe(options: Options, level: Level) -> StaticDoc {
    if level == Level(0) {
        Doc::text("Type")
    } else {
        parens_if(
            Prec::APP < options.prec,
            Doc::text(format!("Type {}", level)),
        )
    }
}

fn pretty_var(options: Options, var: &Var) -> StaticDoc {
    match options.debug_indices {
        true => Doc::text(format!("{:#}", var)),
        false => Doc::as_string(var),
    }
}

fn pretty_name(options: Options, name: &Name) -> StaticDoc {
    // FIXME: pretty names
    match options.debug_indices {
        true => Doc::text(format!("{:#}", name)),
        false => Doc::as_string(name),
    }
}

fn pretty_lam<A: ToDoc, B: ToDoc>(
    options: Options,
    name: &Name,
    ann: Option<&A>,
    body: &B,
) -> StaticDoc {
    parens_if(
        Prec::LAM < options.prec,
        Doc::group(
            Doc::text(r"\")
                .append(Doc::as_string(name))
                .append(match ann.as_ref() {
                    Some(ann) => Doc::space()
                        .append(Doc::text(":"))
                        .append(Doc::space())
                        .append(ann.to_doc(options.with_prec(Prec::PI)).group()),
                    None => Doc::nil(),
                })
                .append(Doc::space())
                .append(Doc::text("=>")),
        ).append(Doc::group(
            Doc::space()
                .append(body.to_doc(options.with_prec(Prec::NO_WRAP)))
                .nest(options.indent_width as usize),
        )),
    )
}

fn pretty_pi<A: ToDoc, B: ToDoc>(options: Options, name: &Name, ann: &A, body: &B) -> StaticDoc {
    parens_if(
        Prec::PI < options.prec,
        Doc::group(
            Doc::text("(")
                .append(Doc::as_string(name))
                .append(Doc::space())
                .append(Doc::text(":"))
                .append(Doc::space())
                .append(ann.to_doc(options.with_prec(Prec::PI)))
                .append(Doc::text(")"))
                .append(Doc::space())
                .append(Doc::text("->")),
        ).append(Doc::group(
            Doc::space()
                .append(body.to_doc(options.with_prec(Prec::NO_WRAP)))
                .nest(options.indent_width as usize),
        )),
    )
}

fn pretty_app<F: ToDoc, A: ToDoc>(options: Options, fn_term: &F, arg_term: &A) -> StaticDoc {
    parens_if(
        Prec::APP < options.prec,
        Doc::nil()
            .append(fn_term.to_doc(options.with_prec(Prec::APP)))
            .append(Doc::space())
            .append(arg_term.to_doc(options.with_prec(Prec::APP))),
    )
}

fn pretty_record_ty(inner: StaticDoc) -> StaticDoc {
    Doc::text("Record")
        .append(Doc::space())
        .append(Doc::text("{"))
        .append(inner)
        .append(Doc::text("}"))
}

fn pretty_record(inner: StaticDoc) -> StaticDoc {
    Doc::text("Record")
        .append(Doc::space())
        .append(Doc::text("{"))
        .append(inner)
        .append(Doc::text("}"))
}

fn pretty_empty_record_ty() -> StaticDoc {
    Doc::text("Record")
        .append(Doc::space())
        .append(Doc::text("{}"))
}

fn pretty_empty_record() -> StaticDoc {
    Doc::text("record")
        .append(Doc::space())
        .append(Doc::text("{}"))
}

fn pretty_proj<E: ToDoc>(options: Options, expr: &E, label: &Label) -> StaticDoc {
    parens_if(
        Prec::APP < options.prec, // Is this precedence right?
        Doc::nil()
            .append(expr.to_doc(options.with_prec(Prec::APP))) // ???
            .append(Doc::text("."))
            .append(pretty_name(options, &label.0)),
    )
}

impl ToDoc for RawConstant {
    fn to_doc(&self, _options: Options) -> StaticDoc {
        match *self {
            RawConstant::String(ref value) => Doc::text(format!("{:?}", value)),
            RawConstant::Char(value) => Doc::text(format!("{:?}", value)),
            RawConstant::Int(value) => Doc::as_string(value),
            RawConstant::Float(value) => Doc::as_string(value),
        }
    }
}

impl ToDoc for Constant {
    fn to_doc(&self, _options: Options) -> StaticDoc {
        match *self {
            Constant::String(ref value) => Doc::text(format!("{:?}", value)),
            Constant::Char(value) => Doc::text(format!("{:?}", value)),
            Constant::U8(value) => Doc::as_string(value),
            Constant::U16(value) => Doc::as_string(value),
            Constant::U32(value) => Doc::as_string(value),
            Constant::U64(value) => Doc::as_string(value),
            Constant::I8(value) => Doc::as_string(value),
            Constant::I16(value) => Doc::as_string(value),
            Constant::I32(value) => Doc::as_string(value),
            Constant::I64(value) => Doc::as_string(value),
            Constant::F32(value) => Doc::as_string(value),
            Constant::F64(value) => Doc::as_string(value),
            Constant::StringType => Doc::text("#String"),
            Constant::CharType => Doc::text("#Char"),
            Constant::U8Type => Doc::text("#U8"),
            Constant::U16Type => Doc::text("#U16"),
            Constant::U32Type => Doc::text("#U32"),
            Constant::U64Type => Doc::text("#U64"),
            Constant::I8Type => Doc::text("#I8"),
            Constant::I16Type => Doc::text("#I16"),
            Constant::I32Type => Doc::text("#I32"),
            Constant::I64Type => Doc::text("#I64"),
            Constant::F32Type => Doc::text("#F32"),
            Constant::F64Type => Doc::text("#F64"),
        }
    }
}

impl ToDoc for RawTerm {
    fn to_doc(&self, options: Options) -> StaticDoc {
        match *self {
            RawTerm::Ann(_, ref expr, ref ty) => pretty_ann(options, expr, ty),
            RawTerm::Universe(_, level) => pretty_universe(options, level),
            RawTerm::Hole(_) => Doc::text("_"),
            RawTerm::Constant(_, ref c) => c.to_doc(options),
            RawTerm::Var(_, ref var) => pretty_var(options, var),
            RawTerm::Lam(_, ref scope) => pretty_lam(
                options,
                &scope.unsafe_pattern.0,
                match *(scope.unsafe_pattern.1).0 {
                    RawTerm::Hole(_) => None,
                    _ => Some(&(scope.unsafe_pattern.1).0),
                },
                &scope.unsafe_body,
            ),
            RawTerm::Pi(_, ref scope) => pretty_pi(
                options,
                &scope.unsafe_pattern.0,
                &(scope.unsafe_pattern.1).0,
                &scope.unsafe_body,
            ),
            RawTerm::App(_, ref f, ref a) => pretty_app(options, f, a),
            RawTerm::RecordType(_, ref scope) => {
                let mut inner = Doc::nil();
                let mut scope = scope;

                loop {
                    inner = inner
                        .append(pretty_name(options, &(scope.unsafe_pattern.0).0))
                        .append(Doc::space())
                        .append(Doc::text(":"))
                        .append(Doc::space())
                        .append((scope.unsafe_pattern.1).0.to_doc(options));

                    match *scope.unsafe_body {
                        RawTerm::RecordType(_, ref next_scope) => scope = next_scope,
                        RawTerm::EmptyRecordType(_) => break,
                        _ => panic!("ill-formed record"),
                    }
                }

                pretty_record_ty(inner)
            },
            RawTerm::Record(_, ref scope) => {
                let mut inner = Doc::nil();
                let mut scope = scope;

                loop {
                    inner = inner
                        .append(pretty_name(options, &(scope.unsafe_pattern.0).0))
                        .append(Doc::space())
                        .append(Doc::text("="))
                        .append(Doc::space())
                        .append((scope.unsafe_pattern.1).0.to_doc(options));

                    match *scope.unsafe_body {
                        RawTerm::Record(_, ref next_scope) => scope = next_scope,
                        RawTerm::EmptyRecord(_) => break,
                        _ => panic!("ill-formed record"),
                    }
                }

                pretty_record(inner)
            },
            RawTerm::EmptyRecordType(_) => pretty_empty_record_ty(),
            RawTerm::EmptyRecord(_) => pretty_empty_record(),
            RawTerm::Proj(_, ref expr, ref label) => pretty_proj(options, expr, label),
        }
    }
}

impl ToDoc for Term {
    fn to_doc(&self, options: Options) -> StaticDoc {
        match *self {
            Term::Ann(_, ref expr, ref ty) => pretty_ann(options, expr, ty),
            Term::Universe(_, level) => pretty_universe(options, level),
            Term::Constant(_, ref c) => c.to_doc(options),
            Term::Var(_, ref var) => pretty_var(options, var),
            Term::Lam(_, ref scope) => pretty_lam(
                options,
                &scope.unsafe_pattern.0,
                Some(&(scope.unsafe_pattern.1).0),
                &scope.unsafe_body,
            ),
            Term::Pi(_, ref scope) => pretty_pi(
                options,
                &scope.unsafe_pattern.0,
                &(scope.unsafe_pattern.1).0,
                &scope.unsafe_body,
            ),
            Term::App(_, ref f, ref a) => pretty_app(options, f, a),
            Term::RecordType(_, ref scope) => {
                let mut inner = Doc::nil();
                let mut scope = scope;

                loop {
                    inner = inner
                        .append(pretty_name(options, &(scope.unsafe_pattern.0).0))
                        .append(Doc::space())
                        .append(Doc::text(":"))
                        .append(Doc::space())
                        .append((scope.unsafe_pattern.1).0.to_doc(options));

                    match *scope.unsafe_body {
                        Term::RecordType(_, ref next_scope) => scope = next_scope,
                        Term::EmptyRecordType(_) => break,
                        _ => panic!("ill-formed record"),
                    }
                }

                pretty_record_ty(inner)
            },
            Term::Record(_, ref scope) => {
                let mut inner = Doc::nil();
                let mut scope = scope;

                loop {
                    inner = inner
                        .append(pretty_name(options, &(scope.unsafe_pattern.0).0))
                        .append(Doc::space())
                        .append(Doc::text("="))
                        .append(Doc::space())
                        .append((scope.unsafe_pattern.1).0.to_doc(options));

                    match *scope.unsafe_body {
                        Term::Record(_, ref next_scope) => scope = next_scope,
                        Term::EmptyRecord(_) => break,
                        _ => panic!("ill-formed record"),
                    }
                }

                pretty_record(inner)
            },
            Term::EmptyRecordType(_) => pretty_empty_record_ty(),
            Term::EmptyRecord(_) => pretty_empty_record(),
            Term::Proj(_, ref expr, ref label) => pretty_proj(options, expr, label),
        }
    }
}

impl ToDoc for Value {
    fn to_doc(&self, options: Options) -> StaticDoc {
        match *self {
            Value::Universe(level) => pretty_universe(options, level),
            Value::Constant(ref c) => c.to_doc(options),
            Value::Lam(ref scope) => pretty_lam(
                options,
                &scope.unsafe_pattern.0,
                Some(&(scope.unsafe_pattern.1).0),
                &scope.unsafe_body,
            ),
            Value::Pi(ref scope) => pretty_pi(
                options,
                &scope.unsafe_pattern.0,
                &(scope.unsafe_pattern.1).0,
                &scope.unsafe_body,
            ),
            Value::RecordType(ref scope) => {
                let mut inner = Doc::nil();
                let mut scope = scope;

                loop {
                    inner = inner
                        .append(pretty_name(options, &(scope.unsafe_pattern.0).0))
                        .append(Doc::space())
                        .append(Doc::text(":"))
                        .append(Doc::space())
                        .append((scope.unsafe_pattern.1).0.to_doc(options));

                    match *scope.unsafe_body {
                        Value::RecordType(ref next_scope) => scope = next_scope,
                        Value::EmptyRecordType => break,
                        _ => panic!("ill-formed record"),
                    }
                }

                pretty_record_ty(inner)
            },
            Value::Record(ref scope) => {
                let mut inner = Doc::nil();
                let mut scope = scope;

                loop {
                    inner = inner
                        .append(pretty_name(options, &(scope.unsafe_pattern.0).0))
                        .append(Doc::space())
                        .append(Doc::text("="))
                        .append(Doc::space())
                        .append((scope.unsafe_pattern.1).0.to_doc(options));

                    match *scope.unsafe_body {
                        Value::Record(ref next_scope) => scope = next_scope,
                        Value::EmptyRecord => break,
                        _ => panic!("ill-formed record"),
                    }
                }

                pretty_record(inner)
            },
            Value::EmptyRecordType => pretty_empty_record_ty(),
            Value::EmptyRecord => pretty_empty_record(),
            Value::Neutral(ref n) => n.to_doc(options),
        }
    }
}

impl ToDoc for Neutral {
    fn to_doc(&self, options: Options) -> StaticDoc {
        match *self {
            Neutral::Var(ref var) => pretty_var(options, var),
            Neutral::App(ref fn_term, ref arg_term) => pretty_app(options, fn_term, arg_term),
            Neutral::Proj(ref expr, ref label) => pretty_proj(options, expr, label),
        }
    }
}

impl ToDoc for Context {
    fn to_doc(&self, options: Options) -> StaticDoc {
        use syntax::core::ContextEntry;

        Doc::text("[")
            .append(Doc::intersperse(
                self.entries.iter().map(|entry| match *entry {
                    ContextEntry::Claim(ref name, ref ty) => Doc::group(
                        pretty_name(options, name).append(
                            Doc::space()
                                .append(Doc::text(":"))
                                .append(Doc::space())
                                .append(ty.to_doc(options.with_prec(Prec::PI)).group()),
                        ),
                    ),
                    ContextEntry::Definition(ref name, ref term) => Doc::group(
                        pretty_name(options, name).append(
                            Doc::space()
                                .append(Doc::text("="))
                                .append(Doc::space())
                                .append(term.to_doc(options.with_prec(Prec::PI)).group()),
                        ),
                    ),
                }),
                Doc::text(",").append(Doc::space()),
            ))
            .append(Doc::text("]"))
    }
}

impl ToDoc for RawDefinition {
    fn to_doc(&self, options: Options) -> StaticDoc {
        match *self.ann {
            RawTerm::Hole(_) => Doc::nil(),
            ref ann => Doc::group(
                Doc::as_string(&self.name)
                    .append(Doc::space())
                    .append(Doc::text(":"))
                    .append(Doc::space())
                    .append(ann.to_doc(options.with_prec(Prec::NO_WRAP)))
                    .append(Doc::text(";")),
            ).append(Doc::newline()),
        }.append(Doc::group(
            Doc::as_string(&self.name)
                .append(Doc::space())
                .append(Doc::text("="))
                .append(Doc::space())
                .append(self.term.to_doc(options.with_prec(Prec::NO_WRAP)))
                .append(Doc::text(";")),
        ))
    }
}

impl ToDoc for RawModule {
    fn to_doc(&self, options: Options) -> StaticDoc {
        Doc::group(
            Doc::text("module")
                .append(Doc::space())
                .append(Doc::as_string(&self.name))
                .append(Doc::text(";")),
        ).append(Doc::newline())
            .append(Doc::newline())
            .append(Doc::intersperse(
                self.definitions
                    .iter()
                    .map(|definition| definition.to_doc(options.with_prec(Prec::NO_WRAP))),
                Doc::newline().append(Doc::newline()),
            ))
    }
}
