//! Pretty prints the core language to a textual form.
use pretty::{DocAllocator, DocBuilder};

use crate::core::{Constant, Term};

pub fn pretty_term<'term, D, S>(alloc: &'term D, term: &'term Term) -> DocBuilder<'term, D>
where
    S: 'term,
    &'term S: ToString,
    D: DocAllocator<'term>,
    D::Doc: Clone,
{
    match term {
        Term::Universe(level) => (alloc.nil())
            .append("Type")
            .append("^")
            .append(alloc.as_string(level.0)),
        Term::Global(name) => alloc.text(name),
        Term::Constant(constant) => pretty_constant(alloc, constant),
        Term::Sequence(term_entries) => (alloc.nil())
            .append("[")
            .group()
            .append(alloc.intersperse(
                term_entries.iter().map(|term| {
                    (alloc.space())
                        .append(pretty_term(alloc, term))
                        .group()
                        .nest(4)
                }),
                alloc.text(",").append(alloc.space()),
            ))
            .append("]"),
        Term::Ann(term, r#type) => (alloc.nil())
            .append(pretty_term(alloc, term))
            .append(alloc.space())
            .append(":")
            .append(
                (alloc.space())
                    .append(pretty_term(alloc, r#type))
                    .group()
                    .nest(4),
            ),
        Term::RecordType(ty_entries) => (alloc.nil())
            .append("Record")
            .append(alloc.space())
            .append("{")
            .group()
            .append(alloc.concat(ty_entries.iter().map(|(name, r#type)| {
                (alloc.nil())
                    .append(alloc.hardline())
                    .append(alloc.as_string(name))
                    .append(":")
                    .append(
                        (alloc.space())
                            .append(pretty_term(alloc, r#type))
                            .append(",")
                            .group()
                            .nest(4),
                    )
                    .nest(4)
                    .group()
            })))
            .append("}"),
        Term::RecordTerm(term_entries) => (alloc.nil())
            .append("record")
            .append(alloc.space())
            .append("{")
            .group()
            .append(alloc.concat(term_entries.iter().map(|(name, term)| {
                (alloc.nil())
                    .append(alloc.hardline())
                    .append(alloc.as_string(name))
                    .append("=")
                    .append(
                        (alloc.space())
                            .append(pretty_term(alloc, term))
                            .append(",")
                            .group()
                            .nest(4),
                    )
                    .nest(4)
                    .group()
            })))
            .append("}"),
        Term::ArrayType(len, entry_type) => alloc.text("Array").append(
            (alloc.nil())
                .append(alloc.space())
                .append(pretty_term(alloc, len))
                .append(alloc.space())
                .append(pretty_term(alloc, entry_type))
                .group()
                .nest(4),
        ),
        Term::ListType(entry_type) => alloc.text("List").append(
            (alloc.nil())
                .append(alloc.space())
                .append(pretty_term(alloc, entry_type))
                .group()
                .nest(4),
        ),
        Term::Lift(term, shift) => (alloc.nil())
            .append(pretty_term(alloc, term))
            .append("^")
            .append(alloc.as_string(shift.0)),
        Term::Error => alloc.text("!"),
    }
}

pub fn pretty_constant<'term, D, S>(
    alloc: &'term D,
    constant: &'term Constant,
) -> DocBuilder<'term, D>
where
    S: 'term,
    &'term S: ToString,
    D: DocAllocator<'term>,
    D::Doc: Clone,
{
    match constant {
        Constant::U8(value) => alloc.as_string(format!("{}", value)),
        Constant::U16(value) => alloc.as_string(format!("{}", value)),
        Constant::U32(value) => alloc.as_string(format!("{}", value)),
        Constant::U64(value) => alloc.as_string(format!("{}", value)),
        Constant::S8(value) => alloc.as_string(format!("{}", value)),
        Constant::S16(value) => alloc.as_string(format!("{}", value)),
        Constant::S32(value) => alloc.as_string(format!("{}", value)),
        Constant::S64(value) => alloc.as_string(format!("{}", value)),
        Constant::F32(value) => alloc.as_string(format!("{}", value)),
        Constant::F64(value) => alloc.as_string(format!("{}", value)),
        Constant::Char(value) => alloc.as_string(format!("{:?}", value)),
        Constant::String(value) => alloc.as_string(format!("{:?}", value)),
    }
}
