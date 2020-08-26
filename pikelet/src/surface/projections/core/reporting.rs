use codespan_reporting::diagnostic::{Diagnostic, Label};
use std::ops::Range;

use crate::surface::Term;

#[derive(Clone, Debug)]
pub enum InvalidLiteral {
    Char,
    String,
    Number,
}

#[derive(Clone, Debug)]
pub enum AmbiguousTerm {
    NumberLiteral,
    Sequence,
    FunctionTerm,
    RecordTerm,
}

#[derive(Clone, Debug)]
pub enum ExpectedType {
    Universe,
    Type(Term<String>),
}

#[derive(Clone, Debug)]
pub enum Message {
    MaximumUniverseLevelReached {
        range: Range<usize>,
    },
    UnboundName {
        range: Range<usize>,
        name: String,
    },
    InvalidRecordType {
        duplicate_names: Vec<(String, Range<usize>, Range<usize>)>,
    },
    InvalidRecordTerm {
        range: Range<usize>,
        duplicate_names: Vec<(String, Range<usize>, Range<usize>)>,
        missing_names: Vec<String>,
        unexpected_names: Vec<(String, Range<usize>)>,
    },
    EntryNameNotFound {
        head_range: Range<usize>,
        name_range: Range<usize>,
        expected_field_name: String,
        head_type: Term<String>,
    },
    TooManyParameters {
        unexpected_parameters: Vec<Range<usize>>,
    },
    TooManyArguments {
        head_range: Range<usize>,
        head_type: Term<String>,
        unexpected_arguments: Vec<Range<usize>>,
    },
    InvalidLiteral {
        range: Range<usize>,
        literal: InvalidLiteral,
    },
    NoLiteralConversion {
        range: Range<usize>,
        expected_type: Term<String>,
    },
    MismatchedSequenceLength {
        range: Range<usize>,
        found_len: usize,
        expected_len: Term<String>,
    },
    NoSequenceConversion {
        range: Range<usize>,
        expected_type: Term<String>,
    },
    AmbiguousTerm {
        range: Range<usize>,
        term: AmbiguousTerm,
    },
    MismatchedTypes {
        range: Range<usize>,
        found_type: Term<String>,
        expected_type: ExpectedType,
    },
}

impl Message {
    pub fn to_diagnostic(&self) -> Diagnostic<()> {
        use itertools::Itertools;

        let pretty_alloc = pretty::BoxAllocator;
        let to_doc = |term| crate::surface::projections::pretty::pretty_term(&pretty_alloc, term).1;

        match self {
            Message::MaximumUniverseLevelReached { range } => Diagnostic::error()
                .with_message("maximum universe level reached")
                .with_labels(vec![
                    Label::primary((), range.clone()).with_message("overflowing universe level")
                ]),

            Message::UnboundName { range, name } => Diagnostic::error()
                .with_message(format!("cannot find `{}` in this scope", name))
                // TODO: name suggestions?
                .with_labels(vec![
                    Label::primary((), range.clone()).with_message("not found in this scope")
                ]),

            Message::InvalidRecordType { duplicate_names } => Diagnostic::error()
                .with_message("invalid record type")
                .with_labels({
                    let mut labels = Vec::with_capacity(duplicate_names.len() * 2);

                    for (name, name_range1, name_range2) in duplicate_names {
                        labels.push(
                            Label::secondary((), name_range1.clone())
                                .with_message(format!("first use of `{}`", name)),
                        );
                        labels.push(
                            Label::primary((), name_range2.clone())
                                .with_message("entry name used more than once"),
                        );
                    }

                    labels
                }),

            Message::InvalidRecordTerm {
                range,
                duplicate_names,
                missing_names,
                unexpected_names,
            } => Diagnostic::error()
                .with_message("invalid record term")
                .with_labels({
                    let mut labels = Vec::with_capacity(
                        duplicate_names.len() * 2
                            + unexpected_names.len()
                            + if missing_names.is_empty() { 0 } else { 1 },
                    );

                    for (name, name_range1, name_range2) in duplicate_names {
                        labels.push(
                            Label::primary((), name_range1.clone())
                                .with_message(format!("first use of `{}`", name)),
                        );
                        labels.push(
                            Label::primary((), name_range2.clone())
                                .with_message("entry name used more than once"),
                        );
                    }

                    for (_, name_range) in unexpected_names {
                        labels.push(
                            Label::primary((), name_range.clone())
                                .with_message("unexpected entry name"),
                        );
                    }

                    if !missing_names.is_empty() {
                        labels.push(Label::primary((), range.clone()).with_message(format!(
                                "missing the names {} in this record term",
                                missing_names
                                    .iter()
                                    // TODO: reduce string allocations
                                    .map(|name| format!("`{}`", name))
                                    .format(", "),
                            )));
                    }

                    labels
                }),

            Message::EntryNameNotFound {
                head_range,
                name_range,
                expected_field_name,
                head_type,
            } => Diagnostic::error()
                .with_message(format!(
                    "no entry named `{}` in type `{}`",
                    expected_field_name,
                    to_doc(&head_type).pretty(std::usize::MAX),
                ))
                .with_labels(vec![
                    Label::primary((), name_range.clone()).with_message("unknown entry name"),
                    Label::secondary((), head_range.clone()).with_message(format!(
                        "the type here is `{}`",
                        to_doc(&head_type).pretty(std::usize::MAX),
                    )),
                ]),

            Message::TooManyParameters {
                unexpected_parameters,
            } => Diagnostic::error()
                .with_message("too many parameters given for function term")
                .with_labels(
                    unexpected_parameters
                        .iter()
                        .map(|parameter_range| {
                            Label::primary((), parameter_range.clone())
                                .with_message("unexpected parameter")
                        })
                        .collect(),
                ),

            Message::TooManyArguments {
                head_range,
                head_type,
                unexpected_arguments,
            } => Diagnostic::error()
                .with_message("term was applied to too many arguments")
                .with_labels(
                    std::iter::once(Label::primary((), head_range.clone()).with_message(format!(
                        // TODO: multi-line?
                        "expected a function, found `{}`",
                        to_doc(&head_type).pretty(std::usize::MAX),
                    )))
                    .chain(unexpected_arguments.iter().map(|argument_range| {
                        Label::primary((), argument_range.clone())
                            .with_message("unexpected argument".to_owned())
                    }))
                    .collect(),
                ),

            Message::InvalidLiteral { range, literal } => Diagnostic::error()
                .with_message(format!(
                    // TODO: supply expected type information
                    "invalid {} literal",
                    match literal {
                        InvalidLiteral::Char => "character",
                        InvalidLiteral::String => "string",
                        InvalidLiteral::Number => "numeric",
                    },
                ))
                .with_labels(vec![
                    Label::primary((), range.clone()).with_message("failed to parse literal")
                ]),

            Message::NoLiteralConversion {
                range,
                expected_type,
            } => Diagnostic::error()
                .with_message("no known literal conversion")
                .with_labels(vec![Label::primary((), range.clone()).with_message(
                    format!(
                        // TODO: multi-line?
                        "expected `{}`, found a literal",
                        to_doc(&expected_type).pretty(std::usize::MAX),
                    ),
                )]),

            Message::MismatchedSequenceLength {
                range,
                found_len,
                expected_len,
            } => Diagnostic::error()
                .with_message("mismatched sequence length")
                .with_labels(vec![Label::primary((), range.clone()).with_message(
                    format!(
                        // TODO: multi-line?
                        "expected `{}` entries, found `{}` entries",
                        to_doc(&expected_len).pretty(std::usize::MAX),
                        found_len,
                    ),
                )]),

            Message::NoSequenceConversion {
                range,
                expected_type,
            } => Diagnostic::error()
                .with_message("no known sequence conversion")
                .with_labels(vec![Label::primary((), range.clone()).with_message(
                    format!(
                        // TODO: multi-line?
                        "expected `{}`, found a sequence",
                        to_doc(&expected_type).pretty(std::usize::MAX),
                    ),
                )]),

            Message::AmbiguousTerm { range, term } => Diagnostic::error()
                .with_message(format!(
                    "ambiguous {}",
                    match term {
                        AmbiguousTerm::NumberLiteral => "numeric literal",
                        AmbiguousTerm::Sequence => "sequence",
                        AmbiguousTerm::FunctionTerm => "function term",
                        AmbiguousTerm::RecordTerm => "record term",
                    },
                ))
                .with_labels(vec![
                    Label::primary((), range.clone()).with_message("type annotations needed")
                ]),

            Message::MismatchedTypes {
                range,
                found_type,
                expected_type,
            } => Diagnostic::error()
                .with_message("mismatched types")
                .with_labels(vec![Label::primary((), range.clone()).with_message(
                    match expected_type {
                        ExpectedType::Universe => format!(
                            // TODO: multi-line?
                            "expected a type, found `{}`",
                            to_doc(&found_type).pretty(std::usize::MAX),
                        ),
                        ExpectedType::Type(expected_type) => format!(
                            // TODO: multi-line?
                            "expected `{}`, found `{}`",
                            to_doc(&expected_type).pretty(std::usize::MAX),
                            to_doc(&found_type).pretty(std::usize::MAX),
                        ),
                    },
                )]),
        }
    }
}
