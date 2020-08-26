//! Elaborates the surface language into the core language.

use std::ops::Range;
use std::str::FromStr;
use std::sync::Arc;

use crate::core;
use crate::surface::{Literal, Term};

/// The state of the elaborator.
pub struct State<'me> {
    /// Global definition environment.
    globals: &'me core::Globals,
    /// The current universe offset.
    universe_offset: core::UniverseOffset,
    /// Substitutions from the user-defined names to the level in which they were bound.
    names_to_levels: Vec<(String, core::LocalLevel)>,
    /// Local name environment (used for pretty printing).
    names: core::Locals<String>,
    /// Local type environment (used for getting the types of local variables).
    types: core::Locals<Arc<core::Value>>,
    /// Local value environment (used for evaluation).
    values: core::Locals<Arc<core::Value>>,
    /// The diagnostic messages accumulated during elaboration.
    messages: Vec<Message>,
}

impl<'me> State<'me> {
    /// Construct a new elaborator state.
    pub fn new(globals: &'me core::Globals) -> State<'me> {
        State {
            globals,
            universe_offset: core::UniverseOffset(0),
            names_to_levels: Vec::new(),
            names: core::Locals::new(),
            types: core::Locals::new(),
            values: core::Locals::new(),
            messages: Vec::new(),
        }
    }

    /// Get the next level to be used for a local entry.
    fn next_level(&self) -> core::LocalLevel {
        self.values.size().next_level()
    }

    /// Get a local entry.
    fn get_local(&self, name: &str) -> Option<(core::LocalIndex, &Arc<core::Value>)> {
        let (_, level) = self.names_to_levels.iter().rev().find(|(n, _)| n == name)?;
        let index = self.values.size().index(*level);
        let ty = self.types.get(index)?;
        Some((index, ty))
    }

    /// Push a local entry.
    fn push_local(&mut self, name: String, value: Arc<core::Value>, r#type: Arc<core::Value>) {
        self.names_to_levels.push((name.clone(), self.next_level()));
        self.names.push(name);
        self.types.push(r#type);
        self.values.push(value);
    }

    /// Push a local parameter.
    fn push_param(&mut self, name: String, r#type: Arc<core::Value>) -> Arc<core::Value> {
        let value = Arc::new(core::Value::local(self.next_level(), r#type.clone()));
        self.push_local(name, value.clone(), r#type);
        value
    }

    // /// Pop a local entry.
    // fn pop_local(&mut self) {
    //     self.names_to_levels.pop();
    //     self.names.pop();
    //     self.types.pop();
    //     self.values.pop();
    // }

    /// Pop the given number of local entries.
    fn pop_many_locals(&mut self, count: usize) {
        self.names_to_levels
            .truncate(self.names_to_levels.len().saturating_sub(count));
        self.names.pop_many(count);
        self.types.pop_many(count);
        self.values.pop_many(count);
    }

    /// Report a diagnostic message.
    fn report(&mut self, error: Message) {
        self.messages.push(error);
    }

    /// Drain the currently accumulated diagnostic messages.
    pub fn drain_messages(&mut self) -> std::vec::Drain<Message> {
        self.messages.drain(..)
    }

    /// Reset the elaborator state while retaining existing allocations.
    pub fn clear(&mut self) {
        self.universe_offset = core::UniverseOffset(0);
        self.names_to_levels.clear();
        self.types.clear();
        self.values.clear();
        self.messages.clear();
    }

    /// Evaluate a term using the current state of the elaborator.
    pub fn eval_term(&mut self, term: &core::Term) -> Arc<core::Value> {
        core::semantics::eval_term(self.globals, self.universe_offset, &mut self.values, term)
    }

    /// Eliminate a closure.
    fn eval_closure_elim(
        &self,
        closure: &core::Closure,
        argument: Arc<core::Value>,
    ) -> Arc<core::Value> {
        core::semantics::eval_closure_elim(self.globals, closure, argument)
    }

    /// Normalize a term using the current state of the elaborator.
    pub fn normalize_term(&mut self, term: &core::Term, r#type: &core::Value) -> core::Term {
        core::semantics::normalize_term(
            self.globals,
            self.universe_offset,
            &mut self.values,
            term,
            r#type,
        )
    }

    /// Read back a type using the current state of the elaborator.
    pub fn read_back_type(&mut self, r#type: &core::Value) -> core::Term {
        core::semantics::read_back_type(self.globals, self.values.size(), r#type)
    }

    /// Check if `value0` is a subtype of `value1`.
    pub fn is_subtype(&self, value0: &core::Value, value1: &core::Value) -> bool {
        core::semantics::is_subtype(self.globals, self.values.size(), value0, value1)
    }

    /// Delaborate a `core::Term` into a `surface::Term`.
    pub fn delaborate_term(&mut self, core_term: &core::Term) -> Term<String> {
        core::projections::surface::delaborate_term(
            &mut core::projections::surface::State::new(&mut self.names),
            &core_term,
        )
    }
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
    DuplicateNamesInRecordTerm {
        range: Range<usize>,
        duplicate_names: Vec<(Range<usize>, String)>,
    },
    DuplicateNamesInRecordType {
        range: Range<usize>,
        duplicate_names: Vec<(Range<usize>, String)>,
    },
    MissingNamesInRecordTerm {
        range: Range<usize>,
        missing_names: Vec<String>,
    },
    UnexpectedNamesInRecordTerm {
        range: Range<usize>,
        unexpected_names: Vec<(Range<usize>, String)>,
    },
    EntryNameNotFound {
        head_range: Range<usize>,
        name_range: Range<usize>,
        expected_field_name: String,
        head_type: Arc<core::Value>,
    },
    TooManyParametersForFunctionTerm {
        excess_parameters: Vec<(Range<usize>, String)>,
        expected_type: Arc<core::Value>,
    },
    NotAFunction {
        range: Range<usize>,
        head_type: Arc<core::Value>,
    },
    InvalidLiteral {
        range: Range<usize>,
        literal: InvalidLiteral,
    },
    NoLiteralConversion {
        range: Range<usize>,
        expected_type: Arc<core::Value>,
    },
    MismatchedSequenceLength {
        range: Range<usize>,
        found_len: usize,
        expected_len: Arc<core::Value>,
    },
    NoSequenceConversion {
        range: Range<usize>,
        expected_type: Arc<core::Value>,
    },
    ExpectedType {
        range: Range<usize>,
        found_type: Arc<core::Value>,
    },
    AmbiguousTerm {
        range: Range<usize>,
        term: AmbiguousTerm,
    },
    MismatchedTypes {
        range: Range<usize>,
        found_type: Arc<core::Value>,
        expected_type: Arc<core::Value>,
    },
}

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

/// Check that a term is a universe and return its level.
pub fn check_type<S: AsRef<str>>(
    state: &mut State<'_>,
    term: &Term<S>,
) -> (core::Term, Option<core::UniverseLevel>) {
    let (core_term, r#type) = synth_term(state, term);
    match r#type.as_ref() {
        core::Value::Universe(level) => (core_term, Some(*level)),
        _ => {
            state.report(Message::ExpectedType {
                range: term.range(),
                found_type: r#type,
            });
            (core::Term::Error, None)
        }
    }
}

/// Check that a term matches the expected type.
pub fn check_term<S: AsRef<str>>(
    state: &mut State<'_>,
    term: &Term<S>,
    expected_type: &Arc<core::Value>,
) -> core::Term {
    match (term, expected_type.as_ref()) {
        (Term::Literal(_, literal), core::Value::Elim(core::Head::Global(name, _), spine, _)) => {
            use crate::core::Constant::*;

            match (literal, name.as_ref(), spine.as_slice()) {
                (Literal::Number(data), "U8", []) => parse_number(state, term.range(), data, U8),
                (Literal::Number(data), "U16", []) => parse_number(state, term.range(), data, U16),
                (Literal::Number(data), "U32", []) => parse_number(state, term.range(), data, U32),
                (Literal::Number(data), "U64", []) => parse_number(state, term.range(), data, U64),
                (Literal::Number(data), "S8", []) => parse_number(state, term.range(), data, S8),
                (Literal::Number(data), "S16", []) => parse_number(state, term.range(), data, S16),
                (Literal::Number(data), "S32", []) => parse_number(state, term.range(), data, S32),
                (Literal::Number(data), "S64", []) => parse_number(state, term.range(), data, S64),
                (Literal::Number(data), "F32", []) => parse_number(state, term.range(), data, F32),
                (Literal::Number(data), "F64", []) => parse_number(state, term.range(), data, F64),
                (Literal::Char(data), "Char", []) => parse_char(state, term.range(), data),
                (Literal::String(data), "String", []) => parse_string(state, term.range(), data),
                (_, _, _) => {
                    state.report(Message::NoLiteralConversion {
                        range: term.range(),
                        expected_type: expected_type.clone(),
                    });
                    core::Term::Error
                }
            }
        }
        (Term::Literal(_, _), core::Value::Error) => core::Term::Error,
        (Term::Literal(_, _), _) => {
            state.report(Message::NoLiteralConversion {
                range: term.range(),
                expected_type: expected_type.clone(),
            });
            core::Term::Error
        }
        (
            Term::Sequence(_, entry_terms),
            core::Value::Elim(core::Head::Global(name, _), elims, _),
        ) => match (name.as_ref(), elims.as_slice()) {
            ("Array", [core::Elim::Function(len, _), core::Elim::Function(core_entry_type, _)]) => {
                let core_entry_terms = entry_terms
                    .iter()
                    .map(|entry_term| Arc::new(check_term(state, entry_term, core_entry_type)))
                    .collect();

                match **len {
                    core::Value::Constant(core::Constant::U32(len))
                        if len as usize == entry_terms.len() =>
                    {
                        core::Term::Sequence(core_entry_terms)
                    }
                    _ => {
                        state.report(Message::MismatchedSequenceLength {
                            range: term.range(),
                            found_len: entry_terms.len(),
                            expected_len: len.clone(),
                        });

                        core::Term::Error
                    }
                }
            }
            ("List", [core::Elim::Function(core_entry_type, _)]) => {
                let core_entry_terms = entry_terms
                    .iter()
                    .map(|entry_term| Arc::new(check_term(state, entry_term, core_entry_type)))
                    .collect();

                core::Term::Sequence(core_entry_terms)
            }
            _ => {
                state.report(Message::NoSequenceConversion {
                    range: term.range(),
                    expected_type: expected_type.clone(),
                });
                core::Term::Error
            }
        },
        (Term::Sequence(_, _), core::Value::Error) => core::Term::Error,
        (Term::Sequence(_, _), _) => {
            state.report(Message::NoSequenceConversion {
                range: term.range(),
                expected_type: expected_type.clone(),
            });
            core::Term::Error
        }
        (Term::RecordTerm(_, term_entries), _) => {
            use std::collections::btree_map::Entry;
            use std::collections::BTreeMap;

            let mut duplicate_names = Vec::new();
            let mut missing_names = Vec::new();

            let mut expected_type = expected_type.clone();
            let mut core_term_entries = BTreeMap::new();
            let mut term_entries = (term_entries.iter()).fold(
                BTreeMap::new(),
                |mut acc, (entry_name_range, entry_name, entry_term)| {
                    let range = entry_name_range.clone();
                    match acc.entry(entry_name.as_ref().to_owned()) {
                        Entry::Vacant(entry) => drop(entry.insert((range, entry_term))),
                        Entry::Occupied(entry) => {
                            duplicate_names.push((range, entry.key().clone()))
                        }
                    }
                    acc
                },
            );

            while let core::Value::RecordTypeExtend(entry_name, entry_type, rest_type) =
                expected_type.as_ref()
            {
                expected_type = match term_entries.remove(entry_name.as_str()) {
                    Some((_, term)) => {
                        let core_term = Arc::new(check_term(state, term, entry_type));
                        let core_value = state.eval_term(&core_term);
                        core_term_entries.insert(entry_name.clone(), core_term);
                        state.eval_closure_elim(rest_type, core_value)
                    }
                    None => {
                        missing_names.push(entry_name.clone());
                        state.eval_closure_elim(rest_type, Arc::new(core::Value::Error))
                    }
                };
            }

            if !duplicate_names.is_empty() {
                state.report(Message::DuplicateNamesInRecordTerm {
                    range: term.range(),
                    duplicate_names,
                });
            }
            if !missing_names.is_empty() {
                state.report(Message::MissingNamesInRecordTerm {
                    range: term.range(),
                    missing_names,
                });
            }
            if !term_entries.is_empty() {
                let unexpected_names = (term_entries.into_iter())
                    .map(|(entry_name, (entry_name_range, _))| (entry_name_range, entry_name))
                    .collect();
                state.report(Message::UnexpectedNamesInRecordTerm {
                    range: term.range(),
                    unexpected_names,
                });
            }

            core::Term::RecordTerm(core_term_entries)
        }
        (Term::FunctionTerm(_, param_names, body), _) => {
            let core_body = {
                let mut expected_type = expected_type;
                let mut param_names = param_names.iter();
                while let Some((_, param_name)) = param_names.next() {
                    match expected_type.as_ref() {
                        core::Value::FunctionType(param_type, body_type) => {
                            state.push_param(param_name.as_ref().to_owned(), param_type.clone());
                            expected_type = body_type;
                        }
                        _ => {
                            state.report(Message::TooManyParametersForFunctionTerm {
                                excess_parameters: param_names
                                    .map(|(param_name_range, param_name)| {
                                        (param_name_range.clone(), param_name.as_ref().to_owned())
                                    })
                                    .collect(),
                                expected_type: expected_type.clone(),
                            });
                            return core::Term::Error;
                        }
                    }
                }
                check_term(state, body, expected_type)
            };
            state.pop_many_locals(param_names.len());
            (param_names.iter().rev()).fold(core_body, |core_body, (_, param_name)| {
                core::Term::FunctionTerm(param_name.as_ref().to_owned(), Arc::new(core_body))
            })
        }
        (term, _) => match synth_term(state, term) {
            (term, found_type) if state.is_subtype(&found_type, expected_type) => term,
            (_, found_type) => {
                state.report(Message::MismatchedTypes {
                    range: term.range(),
                    found_type,
                    expected_type: expected_type.clone(),
                });
                core::Term::Error
            }
        },
    }
}

/// Synthesize the type of a term.
pub fn synth_term<S: AsRef<str>>(
    state: &mut State<'_>,
    term: &Term<S>,
) -> (core::Term, Arc<core::Value>) {
    use std::collections::{BTreeMap, BTreeSet};

    match term {
        Term::Name(_, name) => {
            if let Some((index, r#type)) = state.get_local(name.as_ref()) {
                return (core::Term::Local(index), r#type.clone());
            }

            if let Some((r#type, _)) = state.globals.get(name.as_ref()) {
                let global = core::Term::Global(name.as_ref().to_owned());
                return (global.lift(state.universe_offset), state.eval_term(r#type));
            }

            state.report(Message::UnboundName {
                range: term.range(),
                name: name.as_ref().to_owned(),
            });
            (core::Term::Error, Arc::new(core::Value::Error))
        }
        Term::Ann(term, r#type) => {
            let (core_type, _) = check_type(state, r#type);
            let core_type_value = state.eval_term(&core_type);
            let core_term = check_term(state, term, &core_type_value);
            (
                core::Term::Ann(Arc::new(core_term), Arc::new(core_type)),
                core_type_value,
            )
        }
        Term::Literal(_, literal) => match literal {
            Literal::Number(_) => {
                state.report(Message::AmbiguousTerm {
                    range: term.range(),
                    term: AmbiguousTerm::NumberLiteral,
                });
                (core::Term::Error, Arc::new(core::Value::Error))
            }
            Literal::Char(data) => (
                parse_char(state, term.range(), data),
                Arc::new(core::Value::global("Char", 0, core::Value::universe(0))),
            ),
            Literal::String(data) => (
                parse_string(state, term.range(), data),
                Arc::new(core::Value::global("String", 0, core::Value::universe(0))),
            ),
        },
        Term::Sequence(_, _) => {
            state.report(Message::AmbiguousTerm {
                range: term.range(),
                term: AmbiguousTerm::Sequence,
            });
            (core::Term::Error, Arc::new(core::Value::Error))
        }
        Term::RecordTerm(_, term_entries) => {
            if term_entries.is_empty() {
                (
                    core::Term::RecordTerm(BTreeMap::new()),
                    Arc::from(core::Value::RecordTypeEmpty),
                )
            } else {
                state.report(Message::AmbiguousTerm {
                    range: term.range(),
                    term: AmbiguousTerm::RecordTerm,
                });
                (core::Term::Error, Arc::new(core::Value::Error))
            }
        }
        Term::RecordType(_, type_entries) => {
            let mut max_level = core::UniverseLevel(0);
            let mut duplicate_names = Vec::new();
            let mut seen_names = BTreeSet::new();
            let mut core_type_entries = Vec::new();

            for (entry_name_range, entry_name, entry_type) in type_entries {
                if seen_names.insert(entry_name.as_ref()) {
                    let (core_type, level) = check_type(state, entry_type);
                    max_level = match level {
                        Some(level) => std::cmp::max(max_level, level),
                        None => return (core::Term::Error, Arc::new(core::Value::Error)),
                    };
                    let core_type = Arc::new(core_type);
                    let core_type_value = state.eval_term(&core_type);
                    core_type_entries.push((entry_name.as_ref().to_owned(), core_type));
                    state.push_param(entry_name.as_ref().to_owned(), core_type_value);
                } else {
                    duplicate_names
                        .push((entry_name_range.clone(), entry_name.as_ref().to_owned()));
                    check_type(state, entry_type);
                }
            }

            if !duplicate_names.is_empty() {
                state.report(Message::DuplicateNamesInRecordType {
                    range: term.range(),
                    duplicate_names,
                });
            }

            state.pop_many_locals(seen_names.len());

            (
                core::Term::RecordType(core_type_entries),
                Arc::new(core::Value::Universe(max_level)),
            )
        }
        Term::RecordElim(head, name_range, name) => {
            let (core_head, mut head_type) = synth_term(state, head);
            let core_head = Arc::new(core_head);

            while let core::Value::RecordTypeExtend(current_name, entry_type, rest_type) =
                head_type.as_ref()
            {
                let term = core::Term::RecordElim(core_head.clone(), name.as_ref().to_owned());
                if name.as_ref() == current_name {
                    return (term, entry_type.clone());
                } else {
                    let value = state.eval_term(&term);
                    head_type = state.eval_closure_elim(rest_type, value);
                }
            }

            state.report(Message::EntryNameNotFound {
                head_range: head.range(),
                name_range: name_range.clone(),
                expected_field_name: name.as_ref().to_owned(),
                head_type,
            });
            (core::Term::Error, Arc::new(core::Value::Error))
        }
        Term::FunctionType(param_type, body_type) => {
            match (check_type(state, param_type), check_type(state, body_type)) {
                ((core_param_type, Some(param_level)), (core_body_type, Some(body_level))) => (
                    core::Term::FunctionType(Arc::new(core_param_type), Arc::new(core_body_type)),
                    Arc::new(core::Value::Universe(std::cmp::max(
                        param_level,
                        body_level,
                    ))),
                ),
                (_, _) => (core::Term::Error, Arc::new(core::Value::Error)),
            }
        }
        Term::FunctionTerm(_, _, _) => {
            state.report(Message::AmbiguousTerm {
                range: term.range(),
                term: AmbiguousTerm::FunctionTerm,
            });
            (core::Term::Error, Arc::new(core::Value::Error))
        }
        Term::FunctionElim(head, arguments) => {
            let (mut core_head, mut head_type) = synth_term(state, head);

            for argument in arguments {
                match head_type.as_ref() {
                    core::Value::FunctionType(param_type, body_type) => {
                        core_head = core::Term::FunctionElim(
                            Arc::new(core_head),
                            Arc::new(check_term(state, argument, &param_type)),
                        );
                        head_type = body_type.clone();
                    }
                    core::Value::Error => return (core::Term::Error, Arc::new(core::Value::Error)),
                    _ => {
                        state.report(Message::NotAFunction {
                            range: term.range(),
                            head_type,
                        });
                        return (core::Term::Error, Arc::new(core::Value::Error));
                    }
                }
            }

            (core_head, head_type)
        }
        Term::Lift(_, inner_term, offset) => {
            match state.universe_offset + core::UniverseOffset(*offset) {
                Some(new_offset) => {
                    let previous_offset = std::mem::replace(&mut state.universe_offset, new_offset);
                    let (core_term, r#type) = synth_term(state, inner_term);
                    state.universe_offset = previous_offset;
                    (core_term, r#type)
                }
                None => {
                    state.report(Message::MaximumUniverseLevelReached {
                        range: term.range(),
                    });
                    (core::Term::Error, Arc::new(core::Value::Error))
                }
            }
        }
        Term::Error(_) => (core::Term::Error, Arc::new(core::Value::Error)),
    }
}

fn parse_number<S: AsRef<str>, T: FromStr>(
    state: &mut State<'_>,
    range: Range<usize>,
    data: &S,
    f: impl Fn(T) -> core::Constant,
) -> core::Term {
    // TODO: improve parser (eg. numeric separators, positive sign)
    match data.as_ref().parse() {
        Ok(value) => core::Term::Constant(f(value)),
        Err(_) => {
            state.report(Message::InvalidLiteral {
                range,
                literal: InvalidLiteral::Number,
            });
            core::Term::Error
        }
    }
}

fn parse_char<S: AsRef<str>>(state: &mut State<'_>, range: Range<usize>, data: &S) -> core::Term {
    // TODO: Improve parser (escapes)
    match data.as_ref().chars().nth(1) {
        Some(value) => core::Term::Constant(core::Constant::Char(value)),
        None => {
            state.report(Message::InvalidLiteral {
                range,
                literal: InvalidLiteral::Char,
            });
            core::Term::Error
        }
    }
}

fn parse_string<S: AsRef<str>>(state: &mut State<'_>, range: Range<usize>, data: &S) -> core::Term {
    // TODO: Improve parser (escapes)
    match data.as_ref().get(1..data.as_ref().len() - 1) {
        Some(value) => core::Term::Constant(core::Constant::String(value.to_owned())),
        None => {
            state.report(Message::InvalidLiteral {
                range,
                literal: InvalidLiteral::String,
            });
            core::Term::Error
        }
    }
}
