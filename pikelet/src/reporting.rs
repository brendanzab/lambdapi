//! Reporting diagnostic messages.

use codespan_reporting::diagnostic::{Diagnostic, Label};
use pretty::DocAllocator;
use std::ops::Range;

use crate::lang::core;
use crate::lang::surface;

/// Global diagnostic messages
#[derive(Clone, Debug)]
pub enum Message {
    /// Errors produced during lexing
    Lexer(LexerError),
    /// Errors produced during parsing
    Parse(ParseError),
    /// Messages produced from [`lang::core::typing`]
    ///
    /// [`lang::core::typing`]: crate::lang::core::typing
    CoreTyping(CoreTypingMessage),
    /// Messages produced from [`pass::surface_to_core`]
    ///
    /// [`pass::surface_to_core`]: crate::pass::surface_to_core
    SurfaceToCore(SurfaceToCoreMessage),
}

impl From<LexerError> for Message {
    fn from(error: LexerError) -> Self {
        Message::Lexer(error)
    }
}

impl From<ParseError> for Message {
    fn from(error: ParseError) -> Self {
        Message::Parse(error)
    }
}

impl From<CoreTypingMessage> for Message {
    fn from(message: CoreTypingMessage) -> Self {
        Message::CoreTyping(message)
    }
}

impl From<SurfaceToCoreMessage> for Message {
    fn from(message: SurfaceToCoreMessage) -> Self {
        Message::SurfaceToCore(message)
    }
}

impl Message {
    pub fn from_lalrpop<T: std::fmt::Display>(
        error: lalrpop_util::ParseError<usize, T, LexerError>,
    ) -> Message {
        use lalrpop_util::ParseError::*;

        match error {
            InvalidToken { location } => Message::from(LexerError::InvalidToken {
                range: location..location,
            }),
            UnrecognizedEOF { location, expected } => Message::from(ParseError::UnrecognizedEOF {
                range: location..location,
                expected,
            }),
            UnrecognizedToken {
                token: (start, token, end),
                expected,
            } => Message::from(ParseError::UnrecognizedToken {
                range: start..end,
                token: token.to_string(),
                expected,
            }),
            ExtraToken {
                token: (start, token, end),
            } => Message::from(ParseError::ExtraToken {
                range: start..end,
                token: token.to_string(),
            }),
            User { error } => Message::from(error),
        }
    }

    pub fn to_diagnostic<'a, D>(&'a self, pretty_alloc: &'a D) -> Diagnostic<()>
    where
        D: DocAllocator<'a>,
        D::Doc: Clone,
    {
        match self {
            Message::Lexer(error) => error.to_diagnostic(),
            Message::Parse(error) => error.to_diagnostic(),
            Message::CoreTyping(message) => message.to_diagnostic(pretty_alloc),
            Message::SurfaceToCore(message) => message.to_diagnostic(pretty_alloc),
        }
    }
}

/// Lexer errors
#[derive(Debug, Clone)]
pub enum LexerError {
    InvalidToken { range: Range<usize> },
}

impl LexerError {
    pub fn to_diagnostic(&self) -> Diagnostic<()> {
        match self {
            LexerError::InvalidToken { range } => Diagnostic::error()
                .with_message("invalid token")
                .with_labels(vec![Label::primary((), range.clone())]),
        }
    }
}

/// Parse errors
#[derive(Clone, Debug)]
pub enum ParseError {
    UnrecognizedEOF {
        range: Range<usize>,
        expected: Vec<String>,
    },
    UnrecognizedToken {
        range: Range<usize>,
        token: String,
        expected: Vec<String>,
    },
    ExtraToken {
        range: Range<usize>,
        token: String,
    },
}

impl ParseError {
    pub fn to_diagnostic(&self) -> Diagnostic<()> {
        match self {
            ParseError::UnrecognizedEOF { range, expected } => Diagnostic::error()
                .with_message("unexpected end of file")
                .with_labels(vec![
                    Label::primary((), range.clone()).with_message("unexpected end of file")
                ])
                .with_notes(format_expected(expected).map_or(Vec::new(), |message| vec![message])),
            ParseError::UnrecognizedToken {
                range,
                token,
                expected,
            } => Diagnostic::error()
                .with_message(format!("unexpected token {}", token))
                .with_labels(vec![
                    Label::primary((), range.clone()).with_message("unexpected token")
                ])
                .with_notes(format_expected(expected).map_or(Vec::new(), |message| vec![message])),
            ParseError::ExtraToken { range, token } => Diagnostic::error()
                .with_message(format!("extra token {}", token))
                .with_labels(vec![
                    Label::primary((), range.clone()).with_message("extra token")
                ]),
        }
    }
}

fn format_expected(expected: &[String]) -> Option<String> {
    use itertools::Itertools;

    expected.split_last().map(|items| match items {
        // TODO: Improve token formatting
        (last, []) => format!("expected {}", last),
        (last, expected) => format!("expected {} or {}", expected.iter().format(", "), last),
    })
}

#[derive(Clone, Debug)]
pub enum InvalidLiteral {
    Char,
    String,
    Number,
}

impl InvalidLiteral {
    fn description(&self) -> &'static str {
        match self {
            InvalidLiteral::Char => "character",
            InvalidLiteral::String => "string",
            InvalidLiteral::Number => "numeric",
        }
    }
}

#[derive(Clone, Debug)]
pub enum AmbiguousTerm {
    NumberLiteral,
    Sequence,
    FunctionTerm,
    RecordTerm,
}

impl AmbiguousTerm {
    fn description(&self) -> &'static str {
        match self {
            AmbiguousTerm::NumberLiteral => "numeric literal",
            AmbiguousTerm::Sequence => "sequence",
            AmbiguousTerm::FunctionTerm => "function term",
            AmbiguousTerm::RecordTerm => "record term",
        }
    }
}

#[derive(Clone, Debug)]
pub enum ExpectedType<T> {
    Universe,
    Type(T),
}

/// Message produced from [lang::core::typing]
#[derive(Clone, Debug)]
pub enum CoreTypingMessage {
    MaximumUniverseLevelReached,
    UnboundGlobal {
        name: String,
    },
    UnboundLocal,
    InvalidRecordType {
        duplicate_labels: Vec<String>,
    },
    MismatchedRecordEntryLengths {
        labels_len: usize,
        entries_len: usize,
    },
    InvalidRecordTerm {
        missing_labels: Vec<String>,
        unexpected_labels: Vec<String>,
    },
    LabelNotFound {
        expected_label: String,
        head_type: core::Term,
    },
    TooManyInputsInFunctionTerm,
    TooManyInputsInFunctionElim {
        head_type: core::Term,
    },
    MismatchedSequenceLength {
        found_len: usize,
        expected_len: core::Term,
    },
    NoSequenceConversion {
        expected_type: core::Term,
    },
    AmbiguousTerm {
        term: AmbiguousTerm,
    },
    MismatchedTypes {
        found_type: core::Term,
        expected_type: ExpectedType<core::Term>,
    },
}

impl CoreTypingMessage {
    pub fn to_diagnostic<'a, D>(&'a self, pretty_alloc: &'a D) -> Diagnostic<()>
    where
        D: DocAllocator<'a>,
        D::Doc: Clone,
    {
        use itertools::Itertools;

        use crate::pass::core_to_pretty;

        let to_doc = |term| core_to_pretty::from_term(pretty_alloc, term).1;

        match self {
            CoreTypingMessage::MaximumUniverseLevelReached => {
                Diagnostic::bug().with_message("maximum universe level reached")
            }
            CoreTypingMessage::UnboundGlobal { name } => {
                Diagnostic::bug().with_message(format!("unbound global variable `{}`", name))
            }
            CoreTypingMessage::UnboundLocal => {
                Diagnostic::bug().with_message("unbound local variable")
            }
            CoreTypingMessage::InvalidRecordType { duplicate_labels } => Diagnostic::bug()
                .with_message("invalid record type")
                .with_notes(
                    duplicate_labels
                        .iter()
                        .map(|name| format!("label `{}` was used more than once", name))
                        .collect(),
                ),
            CoreTypingMessage::MismatchedRecordEntryLengths {
                labels_len,
                entries_len,
            } => Diagnostic::bug()
                .with_message("mismatched record entry lengths")
                .with_notes(vec![
                    format!("found {} labels", labels_len),
                    format!("found {} entries", entries_len),
                ]),
            CoreTypingMessage::InvalidRecordTerm {
                missing_labels,
                unexpected_labels,
            } => Diagnostic::bug()
                .with_message("invalid record term")
                .with_notes({
                    let mut notes = Vec::with_capacity(
                        unexpected_labels.len() + if missing_labels.is_empty() { 0 } else { 1 },
                    );

                    for label in unexpected_labels {
                        notes.push(format!("unexpected label `{}`", label));
                    }

                    if !missing_labels.is_empty() {
                        notes.push(format!(
                            "missing the labels {} in this record term",
                            missing_labels
                                .iter()
                                // TODO: reduce string allocations
                                .map(|label| format!("`{}`", label))
                                .format(", "),
                        ));
                    }

                    notes
                }),
            CoreTypingMessage::LabelNotFound {
                expected_label,
                head_type,
            } => Diagnostic::bug()
                .with_message(format!("label `{}` not found", expected_label))
                .with_notes(vec![format!(
                    "eliminating a term of type `{}`",
                    to_doc(head_type).pretty(std::usize::MAX),
                )]),
            CoreTypingMessage::TooManyInputsInFunctionTerm => {
                Diagnostic::bug().with_message("too many inputs in function term")
            }
            CoreTypingMessage::TooManyInputsInFunctionElim { head_type } => Diagnostic::bug()
                .with_message("too many inputs in function elimination")
                .with_notes(vec![format!(
                    "eliminating a term of type `{}`",
                    to_doc(head_type).pretty(std::usize::MAX),
                )]),
            CoreTypingMessage::MismatchedSequenceLength {
                found_len,
                expected_len,
            } => Diagnostic::bug()
                .with_message("mismatched sequence length")
                .with_notes(vec![format!(
                    "expected `{}` entries, found `{}` entries",
                    to_doc(&expected_len).pretty(std::usize::MAX),
                    found_len,
                )]),
            CoreTypingMessage::NoSequenceConversion { expected_type } => Diagnostic::bug()
                .with_message("no known sequence conversion")
                .with_notes(vec![format!(
                    "expected `{}`, found a sequence",
                    to_doc(&expected_type).pretty(std::usize::MAX),
                )]),
            CoreTypingMessage::AmbiguousTerm { term } => {
                Diagnostic::bug().with_message(format!("ambiguous {}", term.description(),))
            }
            CoreTypingMessage::MismatchedTypes {
                found_type,
                expected_type,
            } => Diagnostic::bug()
                .with_message("mismatched types")
                .with_notes(vec![match expected_type {
                    ExpectedType::Universe => format!(
                        "expected a type, found `{}`",
                        to_doc(&found_type).pretty(std::usize::MAX),
                    ),
                    ExpectedType::Type(expected_type) => format!(
                        "expected `{}`, found `{}`",
                        to_doc(&expected_type).pretty(std::usize::MAX),
                        to_doc(&found_type).pretty(std::usize::MAX),
                    ),
                }]),
        }
    }
}

/// Message produced from [pass::surface_to_core]
#[derive(Clone, Debug)]
pub enum SurfaceToCoreMessage {
    MaximumUniverseLevelReached {
        range: Range<usize>,
    },
    UnboundName {
        range: Range<usize>,
        name: String,
    },
    InvalidRecordType {
        duplicate_labels: Vec<(String, Range<usize>, Range<usize>)>,
    },
    InvalidRecordTerm {
        range: Range<usize>,
        duplicate_labels: Vec<(String, Range<usize>, Range<usize>)>,
        missing_labels: Vec<String>,
        unexpected_labels: Vec<(String, Range<usize>)>,
    },
    LabelNotFound {
        head_range: Range<usize>,
        label_range: Range<usize>,
        expected_label: String,
        head_type: surface::Term,
    },
    TooManyInputsInFunctionTerm {
        unexpected_inputs: Vec<Range<usize>>,
    },
    TooManyInputsInFunctionElim {
        head_range: Range<usize>,
        head_type: surface::Term,
        unexpected_input_terms: Vec<Range<usize>>,
    },
    InvalidLiteral {
        range: Range<usize>,
        literal: InvalidLiteral,
    },
    NoLiteralConversion {
        range: Range<usize>,
        expected_type: surface::Term,
    },
    MismatchedSequenceLength {
        range: Range<usize>,
        found_len: usize,
        expected_len: surface::Term,
    },
    NoSequenceConversion {
        range: Range<usize>,
        expected_type: surface::Term,
    },
    AmbiguousTerm {
        range: Range<usize>,
        term: AmbiguousTerm,
    },
    MismatchedTypes {
        range: Range<usize>,
        found_type: surface::Term,
        expected_type: ExpectedType<surface::Term>,
    },
}

impl SurfaceToCoreMessage {
    pub fn to_diagnostic<'a, D>(&'a self, pretty_alloc: &'a D) -> Diagnostic<()>
    where
        D: DocAllocator<'a>,
        D::Doc: Clone,
    {
        use itertools::Itertools;

        use crate::pass::surface_to_pretty;

        let to_doc = |term| surface_to_pretty::from_term(pretty_alloc, term).1;

        match self {
            SurfaceToCoreMessage::MaximumUniverseLevelReached { range } => Diagnostic::error()
                .with_message("maximum universe level reached")
                .with_labels(vec![
                    Label::primary((), range.clone()).with_message("overflowing universe level")
                ]),

            SurfaceToCoreMessage::UnboundName { range, name } => Diagnostic::error()
                .with_message(format!("cannot find `{}` in this scope", name))
                // TODO: name suggestions?
                .with_labels(vec![
                    Label::primary((), range.clone()).with_message("not found in this scope")
                ]),

            SurfaceToCoreMessage::InvalidRecordType { duplicate_labels } => Diagnostic::error()
                .with_message("invalid record type")
                .with_labels({
                    let mut labels = Vec::with_capacity(duplicate_labels.len() * 2);

                    for (label, label_range1, label_range2) in duplicate_labels {
                        labels.push(
                            Label::secondary((), label_range1.clone())
                                .with_message(format!("first use of `{}`", label)),
                        );
                        labels.push(
                            Label::primary((), label_range2.clone())
                                .with_message("entry label used more than once"),
                        );
                    }

                    labels
                }),

            SurfaceToCoreMessage::InvalidRecordTerm {
                range,
                duplicate_labels,
                missing_labels,
                unexpected_labels,
            } => Diagnostic::error()
                .with_message("invalid record term")
                .with_labels({
                    let mut labels = Vec::with_capacity(
                        duplicate_labels.len() * 2
                            + unexpected_labels.len()
                            + if missing_labels.is_empty() { 0 } else { 1 },
                    );

                    for (label, label_range1, label_range2) in duplicate_labels {
                        labels.push(
                            Label::primary((), label_range1.clone())
                                .with_message(format!("first use of `{}`", label)),
                        );
                        labels.push(
                            Label::primary((), label_range2.clone())
                                .with_message("entry label used more than once"),
                        );
                    }

                    for (_, label_range) in unexpected_labels {
                        labels.push(
                            Label::primary((), label_range.clone())
                                .with_message("unexpected entry label"),
                        );
                    }

                    if !missing_labels.is_empty() {
                        labels.push(Label::primary((), range.clone()).with_message(format!(
                                "missing the labels {} in this record term",
                                missing_labels
                                    .iter()
                                    // TODO: reduce string allocations
                                    .map(|label| format!("`{}`", label))
                                    .format(", "),
                            )));
                    }

                    labels
                }),

            SurfaceToCoreMessage::LabelNotFound {
                head_range,
                label_range,
                expected_label,
                head_type,
            } => Diagnostic::error()
                .with_message(format!(
                    "no entry with label `{}` in type `{}`",
                    expected_label,
                    to_doc(&head_type).pretty(std::usize::MAX),
                ))
                .with_labels(vec![
                    Label::primary((), label_range.clone()).with_message("unknown entry label"),
                    Label::secondary((), head_range.clone()).with_message(format!(
                        "the type here is `{}`",
                        to_doc(&head_type).pretty(std::usize::MAX),
                    )),
                ]),

            SurfaceToCoreMessage::TooManyInputsInFunctionTerm { unexpected_inputs } => {
                Diagnostic::error()
                    .with_message("too many inputs given for function term")
                    .with_labels(
                        unexpected_inputs
                            .iter()
                            .map(|input_range| {
                                Label::primary((), input_range.clone())
                                    .with_message("unexpected input")
                            })
                            .collect(),
                    )
            }

            SurfaceToCoreMessage::TooManyInputsInFunctionElim {
                head_range,
                head_type,
                unexpected_input_terms,
            } => Diagnostic::error()
                .with_message("term was applied to too many inputs")
                .with_labels(
                    std::iter::once(Label::primary((), head_range.clone()).with_message(format!(
                        // TODO: multi-line?
                        "expected a function, found `{}`",
                        to_doc(&head_type).pretty(std::usize::MAX),
                    )))
                    .chain(unexpected_input_terms.iter().map(|input_range| {
                        Label::primary((), input_range.clone())
                            .with_message("unexpected input".to_owned())
                    }))
                    .collect(),
                ),

            SurfaceToCoreMessage::InvalidLiteral { range, literal } => Diagnostic::error()
                // TODO: supply expected type information
                .with_message(format!("invalid {} literal", literal.description()))
                .with_labels(vec![
                    Label::primary((), range.clone()).with_message("failed to parse literal")
                ]),

            SurfaceToCoreMessage::NoLiteralConversion {
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

            SurfaceToCoreMessage::MismatchedSequenceLength {
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

            SurfaceToCoreMessage::NoSequenceConversion {
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

            SurfaceToCoreMessage::AmbiguousTerm { range, term } => Diagnostic::error()
                .with_message(format!("ambiguous {}", term.description()))
                .with_labels(vec![
                    Label::primary((), range.clone()).with_message("type annotations needed")
                ]),

            SurfaceToCoreMessage::MismatchedTypes {
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
