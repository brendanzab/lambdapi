//! Pretty printing for the concrete syntax

use super::pretty::Doc;

use concrete::{Item, LamParamGroup, Literal, Pattern, PiParamGroup, RecordField, Term};
use {FloatFormat, IntFormat};

use super::{StaticDoc, ToDoc};

const INDENT_WIDTH: usize = 4;

impl ToDoc for Item {
    fn to_doc(&self) -> StaticDoc {
        match *self {
            Item::Declaration {
                name: (_, ref name),
                ref ann,
                ..
            } => Doc::as_string(name)
                .append(Doc::space())
                .append(":")
                .append(Doc::space())
                .append(ann.to_doc()),
            Item::Definition {
                name: (_, ref name),
                ref params,
                ref return_ann,
                ref body,
            } => Doc::as_string(name)
                .append(Doc::space())
                .append(match params[..] {
                    [] => Doc::nil(),
                    _ => pretty_lam_params(params).append(Doc::space()),
                })
                .append(return_ann.as_ref().map_or(Doc::nil(), |return_ann| {
                    Doc::text(":")
                        .append(return_ann.to_doc())
                        .append(Doc::space())
                }))
                .append("=")
                .append(Doc::space())
                .append(body.to_doc().nest(INDENT_WIDTH)),
            Item::Error(_) => Doc::text("<error>"),
        }
        .append(";")
    }
}

impl ToDoc for Literal {
    fn to_doc(&self) -> StaticDoc {
        match *self {
            Literal::String(_, ref value) => Doc::text(format!("{:?}", value)),
            Literal::Char(_, value) => Doc::text(format!("{:?}", value)),
            Literal::Int(_, value, IntFormat::Bin) => Doc::text(format!("0b{:b}", value)),
            Literal::Int(_, value, IntFormat::Oct) => Doc::text(format!("0o{:o}", value)),
            Literal::Int(_, value, IntFormat::Dec) => Doc::text(format!("{}", value)),
            Literal::Int(_, value, IntFormat::Hex) => Doc::text(format!("0x{:x}", value)),
            Literal::Float(_, value, FloatFormat::Dec) => Doc::text(format!("{}", value)),
        }
    }
}

impl ToDoc for Pattern {
    fn to_doc(&self) -> StaticDoc {
        match *self {
            Pattern::Parens(_, ref term) => Doc::text("(").append(term.to_doc()).append(")"),
            Pattern::Ann(ref term, ref ty) => term
                .to_doc()
                .append(Doc::space())
                .append(":")
                .append(Doc::space())
                .append(ty.to_doc()),
            Pattern::Name(_, ref name, None) => Doc::text(format!("{}", name)),
            Pattern::Name(_, ref name, Some(shift)) => Doc::text(format!("{}^{}", name, shift)),
            Pattern::Literal(ref literal) => literal.to_doc(),
            Pattern::Error(_) => Doc::text("<error>"),
        }
    }
}

impl ToDoc for Term {
    fn to_doc(&self) -> StaticDoc {
        match *self {
            Term::Parens(_, ref term) => Doc::text("(").append(term.to_doc()).append(")"),
            Term::Ann(ref term, ref ty) => term
                .to_doc()
                .append(Doc::space())
                .append(":")
                .append(Doc::space())
                .append(ty.to_doc()),
            Term::Universe(_, None) => Doc::text("Type"),
            Term::Universe(_, Some(level)) => Doc::text(format!("Type^{}", level)),
            Term::Literal(ref literal) => literal.to_doc(),
            Term::Array(_, ref elems) => Doc::text("[")
                .append(Doc::intersperse(
                    elems.iter().map(Term::to_doc),
                    Doc::text(";").append(Doc::space()),
                ))
                .append("]"),
            Term::Hole(_) => Doc::text("_"),
            Term::Name(_, ref name, None) => Doc::text(format!("{}", name)),
            Term::Name(_, ref name, Some(shift)) => Doc::text(format!("{}^{}", name, shift)),
            Term::Import(_, _, ref name) => Doc::text("import")
                .append(Doc::space())
                .append(format!("{:?}", name)),
            Term::Lam(_, ref params, ref body) => Doc::text("\\")
                .append(pretty_lam_params(params))
                .append(Doc::space())
                .append("=>")
                .append(Doc::space())
                .append(body.to_doc()),
            Term::Pi(_, ref params, ref body) => pretty_pi_params(params)
                .append(Doc::space())
                .append("->")
                .append(Doc::space())
                .append(body.to_doc()),
            Term::Arrow(ref ann, ref body) => ann
                .to_doc()
                .append(Doc::space())
                .append("->")
                .append(Doc::space())
                .append(body.to_doc()),
            Term::App(ref head, ref args) => head.to_doc().append(Doc::space()).append(
                Doc::intersperse(args.iter().map(|arg| arg.to_doc()), Doc::space()),
            ),
            Term::Let(_, ref items, ref body) => {
                Doc::text("let")
                    .append(Doc::space())
                    .append(Doc::intersperse(
                        // FIXME: Indentation
                        items.iter().map(|item| item.to_doc()),
                        Doc::newline(),
                    ))
                    .append("in")
                    .append(body.to_doc())
            },
            Term::Where(ref expr, ref items, _) => expr
                .to_doc()
                .append(Doc::newline())
                .append("where {")
                .append(Doc::newline())
                .append(Doc::intersperse(
                    items.iter().map(|item| item.to_doc().group()),
                    Doc::newline(),
                ))
                .append(Doc::newline())
                .nest(INDENT_WIDTH)
                .append("}"),
            Term::If(_, ref cond, ref if_true, ref if_false) => Doc::text("if")
                .append(Doc::space())
                .append(cond.to_doc())
                .append(Doc::space())
                .append("then")
                .append(Doc::space())
                .append(if_true.to_doc())
                .append(Doc::space())
                .append("else")
                .append(Doc::space())
                .append(if_false.to_doc()),
            Term::Case(_, ref head, ref clauses) => Doc::text("case")
                .append(Doc::space())
                .append(head.to_doc())
                .append(Doc::space())
                .append("of")
                .append(Doc::space())
                .append("{")
                .append(Doc::newline())
                .append(Doc::intersperse(
                    clauses.iter().map(|&(ref pattern, ref body)| {
                        pattern
                            .to_doc()
                            .append(Doc::space())
                            .append("=>")
                            .append(Doc::space())
                            .append(body.to_doc())
                            .append(";")
                    }),
                    Doc::newline(),
                ))
                .append(Doc::newline())
                .nest(INDENT_WIDTH)
                .append("}"),
            Term::RecordType(_, ref fields) if fields.is_empty() => Doc::text("Record {}"),
            Term::Record(_, ref fields) if fields.is_empty() => Doc::text("record {}"),
            Term::RecordType(_, ref fields) => Doc::text("Record {")
                .append(Doc::space())
                .append(Doc::intersperse(
                    fields.iter().map(|field| {
                        Doc::group(
                            Doc::as_string(&field.label.1)
                                .append(match field.binder {
                                    Some((_, ref binder)) => Doc::space()
                                        .append("as")
                                        .append(Doc::space())
                                        .append(Doc::as_string(binder)),
                                    None => Doc::nil(),
                                })
                                .append(Doc::space())
                                .append(":")
                                .append(Doc::space())
                                .append(field.ann.to_doc()),
                        )
                    }),
                    Doc::text(";").append(Doc::space()),
                ))
                .nest(INDENT_WIDTH)
                .append(Doc::space())
                .append("}"),
            Term::Record(_, ref fields) => Doc::text("record {")
                .append(Doc::space())
                .append(Doc::intersperse(
                    fields.iter().map(|field| match field {
                        RecordField::Punned {
                            label: (_, ref label),
                            shift,
                        } => match shift {
                            None => Doc::text(format!("{}", label)),
                            Some(shift) => Doc::text(format!("{}^{}", label, shift)),
                        },
                        RecordField::Explicit {
                            label: (_, ref label),
                            ref params,
                            ref return_ann,
                            ref term,
                        } => Doc::group(
                            Doc::as_string(label)
                                .append(Doc::space())
                                .append(match params[..] {
                                    [] => Doc::nil(),
                                    _ => pretty_lam_params(params).append(Doc::space()),
                                })
                                .append(return_ann.as_ref().map_or(Doc::nil(), |return_ann| {
                                    Doc::text(":")
                                        .append(return_ann.to_doc())
                                        .append(Doc::space())
                                }))
                                .append("=")
                                .append(Doc::space())
                                .append(term.to_doc()),
                        ),
                    }),
                    Doc::text(";").append(Doc::space()),
                ))
                .nest(INDENT_WIDTH)
                .append(Doc::space())
                .append("}"),
            Term::Proj(_, ref expr, _, ref label, None) => {
                expr.to_doc().append(".").append(format!("{}", label))
            },
            Term::Proj(_, ref expr, _, ref label, Some(shift)) => expr
                .to_doc()
                .append(".")
                .append(format!("{}^{}", label, shift)),
            Term::VariantType(_, ref fields) => {
                unimplemented!("pretty concrete::Term::VariantType")
            },
            Term::Variant(_, ref name, ref term) => {
                unimplemented!("pretty concrete::Term::Variant")
            },
            Term::Error(_) => Doc::text("<error>"),
        }
    }
}

fn pretty_lam_params(params: &[LamParamGroup]) -> StaticDoc {
    Doc::intersperse(
        params.iter().map(|&(ref names, ref ann)| match *ann {
            None if names.len() == 1 => Doc::as_string(&names[0].1),
            None => unreachable!(), // FIXME - shouldn't be possible in AST
            Some(ref ann) => Doc::text("(")
                .append(Doc::intersperse(
                    names.iter().map(|name| Doc::as_string(&name.1)),
                    Doc::space(),
                ))
                .append(Doc::space())
                .append(":")
                .append(Doc::space())
                .append(ann.to_doc())
                .append(")"),
        }),
        Doc::space(),
    )
}

fn pretty_pi_params(params: &[PiParamGroup]) -> StaticDoc {
    Doc::intersperse(
        params.iter().map(|&(ref names, ref ann)| {
            Doc::text("(")
                .append(Doc::intersperse(
                    names.iter().map(|name| Doc::as_string(&name.1)),
                    Doc::space(),
                ))
                .append(Doc::space())
                .append(":")
                .append(Doc::space())
                .append(ann.to_doc())
                .append(")")
        }),
        Doc::space(),
    )
}
