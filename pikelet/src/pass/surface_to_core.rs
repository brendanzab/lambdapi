//! Elaborates the surface language into the core language.

use std::ops::Range;
use std::str::FromStr;
use std::sync::Arc;

use crate::lang::core;
use crate::lang::core::semantics::{self, Elim, Head, RecordTypeClosure, Value};
use crate::lang::surface::{Literal, Term};
use crate::pass::core_to_surface;

pub mod reporting;

pub use self::reporting::*;

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
    types: core::Locals<Arc<Value>>,
    /// Local value environment (used for evaluation).
    values: core::Locals<Arc<Value>>,
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
    fn get_local(&self, name: &str) -> Option<(core::LocalIndex, &Arc<Value>)> {
        let (_, level) = self.names_to_levels.iter().rev().find(|(n, _)| n == name)?;
        let index = self.values.size().index(*level);
        let ty = self.types.get(index)?;
        Some((index, ty))
    }

    /// Push a local entry.
    fn push_local(&mut self, name: String, value: Arc<Value>, r#type: Arc<Value>) {
        self.names_to_levels.push((name.clone(), self.next_level()));
        self.names.push(name);
        self.types.push(r#type);
        self.values.push(value);
    }

    /// Push a local parameter.
    fn push_param(&mut self, name: String, r#type: Arc<Value>) -> Arc<Value> {
        let value = Arc::new(Value::local(self.next_level(), r#type.clone()));
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
    pub fn eval_term(&mut self, term: &core::Term) -> Arc<Value> {
        semantics::eval_term(self.globals, self.universe_offset, &mut self.values, term)
    }

    /// Return the type of the record elimination.
    pub fn record_elim_type(
        &mut self,
        head_value: &Value,
        name: &str,
        closure: &RecordTypeClosure,
    ) -> Option<Arc<Value>> {
        semantics::record_elim_type(self.globals, head_value, name, closure)
    }

    /// Normalize a term using the current state of the elaborator.
    pub fn normalize_term(&mut self, term: &core::Term, r#type: &Value) -> core::Term {
        semantics::normalize_term(
            self.globals,
            self.universe_offset,
            &mut self.values,
            term,
            r#type,
        )
    }

    /// Read back a normal form using the current state of the elaborator.
    pub fn read_back_nf(&mut self, value: &Value, r#type: &Value) -> core::Term {
        semantics::read_back_nf(self.globals, self.values.size(), value, r#type)
    }

    /// Read back a type using the current state of the elaborator.
    pub fn read_back_type(&mut self, r#type: &Value) -> core::Term {
        semantics::read_back_type(self.globals, self.values.size(), r#type)
    }

    /// Check if `value0` is a subtype of `value1`.
    pub fn is_subtype(&self, value0: &Value, value1: &Value) -> bool {
        semantics::is_subtype(self.globals, self.values.size(), value0, value1)
    }

    /// Delaborate a `core::Term` into a `surface::Term`.
    pub fn delaborate_term(&mut self, core_term: &core::Term) -> Term<String> {
        core_to_surface::delaborate_term(
            &mut core_to_surface::State::new(&mut self.names),
            &core_term,
        )
    }
}

/// Check that a term is a type return, and return the elaborated term and the
/// universe level it inhabits.
pub fn is_type<S: AsRef<str>>(
    state: &mut State<'_>,
    term: &Term<S>,
) -> (core::Term, Option<core::UniverseLevel>) {
    let (core_term, r#type) = synth_type(state, term);
    match r#type.as_ref() {
        Value::Universe(level) => (core_term, Some(*level)),
        Value::Error => (core::Term::Error, None),
        found_type => {
            let found_type = state.read_back_type(&found_type);
            let found_type = state.delaborate_term(&found_type);
            state.report(Message::MismatchedTypes {
                range: term.range(),
                found_type,
                expected_type: ExpectedType::Universe,
            });
            (core::Term::Error, None)
        }
    }
}

/// Check that a term is an element of a type, and return the elaborated term.
pub fn check_type<S: AsRef<str>>(
    state: &mut State<'_>,
    term: &Term<S>,
    expected_type: &Arc<Value>,
) -> core::Term {
    match (term, expected_type.as_ref()) {
        (_, Value::Error) => core::Term::Error,
        (Term::Literal(_, literal), Value::Elim(Head::Global(name, _), spine, _)) => {
            use crate::lang::core::Constant::*;

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
                    let expected_type = state.read_back_type(expected_type);
                    let expected_type = state.delaborate_term(&expected_type);
                    state.report(Message::NoLiteralConversion {
                        range: term.range(),
                        expected_type,
                    });
                    core::Term::Error
                }
            }
        }
        (Term::Literal(_, _), _) => {
            let expected_type = state.read_back_type(expected_type);
            let expected_type = state.delaborate_term(&expected_type);
            state.report(Message::NoLiteralConversion {
                range: term.range(),
                expected_type,
            });
            core::Term::Error
        }
        (Term::Sequence(_, entry_terms), Value::Elim(Head::Global(name, _), elims, _)) => {
            match (name.as_ref(), elims.as_slice()) {
                ("Array", [Elim::Function(len, len_type), Elim::Function(core_entry_type, _)]) => {
                    let core_entry_terms = entry_terms
                        .iter()
                        .map(|entry_term| Arc::new(check_type(state, entry_term, core_entry_type)))
                        .collect();

                    match **len {
                        Value::Constant(core::Constant::U32(len))
                            if len as usize == entry_terms.len() =>
                        {
                            core::Term::Sequence(core_entry_terms)
                        }
                        Value::Error => core::Term::Error,
                        _ => {
                            let expected_len = state.read_back_nf(&len, len_type);
                            let expected_len = state.delaborate_term(&expected_len);
                            state.report(Message::MismatchedSequenceLength {
                                range: term.range(),
                                found_len: entry_terms.len(),
                                expected_len,
                            });

                            core::Term::Error
                        }
                    }
                }
                ("List", [Elim::Function(core_entry_type, _)]) => {
                    let core_entry_terms = entry_terms
                        .iter()
                        .map(|entry_term| Arc::new(check_type(state, entry_term, core_entry_type)))
                        .collect();

                    core::Term::Sequence(core_entry_terms)
                }
                _ => {
                    let expected_type = state.read_back_type(expected_type);
                    let expected_type = state.delaborate_term(&expected_type);
                    state.report(Message::NoSequenceConversion {
                        range: term.range(),
                        expected_type,
                    });
                    core::Term::Error
                }
            }
        }
        (Term::Sequence(_, _), _) => {
            let expected_type = state.read_back_type(expected_type);
            let expected_type = state.delaborate_term(&expected_type);
            state.report(Message::NoSequenceConversion {
                range: term.range(),
                expected_type,
            });
            core::Term::Error
        }
        (Term::RecordTerm(_, term_entries), Value::RecordType(closure)) => {
            use std::collections::btree_map::Entry;
            use std::collections::BTreeMap;

            let mut duplicate_names = Vec::new();
            let mut missing_names = Vec::new();

            let mut core_term_entries = BTreeMap::new();
            let mut pending_term_entries = (term_entries.iter()).fold(
                BTreeMap::new(),
                |mut acc, (entry_name_range, entry_name, entry_term)| {
                    let range = entry_name_range.clone();
                    match acc.entry(entry_name.as_ref().to_owned()) {
                        Entry::Vacant(entry) => drop(entry.insert((range, entry_term))),
                        Entry::Occupied(entry) => duplicate_names.push((
                            entry.key().clone(),
                            entry.get().0.clone(),
                            range,
                        )),
                    }
                    acc
                },
            );

            closure.entries(
                state.globals,
                |entry_name, entry_type| match pending_term_entries.remove(entry_name) {
                    Some((_, entry_term)) => {
                        let core_entry_term = check_type(state, entry_term, &entry_type);
                        let core_entry_value = state.eval_term(&core_entry_term);
                        core_term_entries.insert(entry_name.to_owned(), Arc::new(core_entry_term));
                        core_entry_value
                    }
                    None => {
                        missing_names.push(entry_name.to_owned());
                        Arc::new(Value::Error)
                    }
                },
            );

            if !duplicate_names.is_empty()
                || !missing_names.is_empty()
                || !pending_term_entries.is_empty()
            {
                let unexpected_names = (pending_term_entries.into_iter())
                    .map(|(entry_name, (entry_name_range, _))| (entry_name, entry_name_range))
                    .collect();
                state.report(Message::InvalidRecordTerm {
                    range: term.range(),
                    duplicate_names,
                    missing_names,
                    unexpected_names,
                });
            }

            core::Term::RecordTerm(core_term_entries)
        }
        (Term::FunctionTerm(_, param_names, body), _) => {
            let mut seen_param_count = 0;
            let mut expected_type = expected_type;
            let mut pending_param_names = param_names.iter();

            while let Some((param_range, param_name)) = pending_param_names.next() {
                match expected_type.as_ref() {
                    Value::FunctionType(param_type, body_type) => {
                        state.push_param(param_name.as_ref().to_owned(), param_type.clone());
                        seen_param_count += 1;
                        expected_type = body_type;
                    }
                    Value::Error => {
                        state.pop_many_locals(seen_param_count);
                        return core::Term::Error;
                    }
                    _ => {
                        state.report(Message::TooManyParameters {
                            unexpected_parameters: std::iter::once(param_range.clone())
                                .chain(pending_param_names.map(|(range, _)| range.clone()))
                                .collect(),
                        });
                        check_type(state, body, expected_type);
                        state.pop_many_locals(seen_param_count);
                        return core::Term::Error;
                    }
                }
            }

            let core_body = check_type(state, body, expected_type);
            state.pop_many_locals(seen_param_count);
            (param_names.iter().rev()).fold(core_body, |core_body, (_, param_name)| {
                core::Term::FunctionTerm(param_name.as_ref().to_owned(), Arc::new(core_body))
            })
        }
        (term, _) => match synth_type(state, term) {
            (term, found_type) if state.is_subtype(&found_type, expected_type) => term,
            (_, found_type) => {
                let found_type = state.read_back_type(&found_type);
                let found_type = state.delaborate_term(&found_type);
                let expected_type = state.read_back_type(expected_type);
                let expected_type = state.delaborate_term(&expected_type);
                state.report(Message::MismatchedTypes {
                    range: term.range(),
                    found_type,
                    expected_type: ExpectedType::Type(expected_type),
                });
                core::Term::Error
            }
        },
    }
}

/// Synthesize the type of a surface term, and return the elaborated term.
pub fn synth_type<S: AsRef<str>>(
    state: &mut State<'_>,
    term: &Term<S>,
) -> (core::Term, Arc<Value>) {
    use std::collections::BTreeMap;

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
            (core::Term::Error, Arc::new(Value::Error))
        }
        Term::Ann(term, r#type) => {
            let (core_type, _) = is_type(state, r#type);
            let core_type_value = state.eval_term(&core_type);
            let core_term = check_type(state, term, &core_type_value);
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
                (core::Term::Error, Arc::new(Value::Error))
            }
            Literal::Char(data) => (
                parse_char(state, term.range(), data),
                Arc::new(Value::global("Char", 0, Value::universe(0))),
            ),
            Literal::String(data) => (
                parse_string(state, term.range(), data),
                Arc::new(Value::global("String", 0, Value::universe(0))),
            ),
        },
        Term::Sequence(_, _) => {
            state.report(Message::AmbiguousTerm {
                range: term.range(),
                term: AmbiguousTerm::Sequence,
            });
            (core::Term::Error, Arc::new(Value::Error))
        }
        Term::RecordTerm(_, term_entries) => {
            if term_entries.is_empty() {
                (
                    core::Term::RecordTerm(BTreeMap::new()),
                    Arc::from(Value::RecordType(RecordTypeClosure::new(
                        state.universe_offset,
                        state.values.clone(),
                        Arc::new([]),
                    ))),
                )
            } else {
                state.report(Message::AmbiguousTerm {
                    range: term.range(),
                    term: AmbiguousTerm::RecordTerm,
                });
                (core::Term::Error, Arc::new(Value::Error))
            }
        }
        Term::RecordType(_, type_entries) => {
            use std::collections::btree_map::Entry;

            let mut max_level = core::UniverseLevel(0);
            let mut duplicate_names = Vec::new();
            let mut seen_names = BTreeMap::new();
            let mut core_type_entries = Vec::new();

            for (entry_name_range, entry_name, entry_type) in type_entries {
                match seen_names.entry(entry_name.as_ref()) {
                    Entry::Vacant(entry) => {
                        let (core_type, level) = is_type(state, entry_type);
                        max_level = match level {
                            Some(level) => std::cmp::max(max_level, level),
                            None => {
                                state.pop_many_locals(seen_names.len());
                                return (core::Term::Error, Arc::new(Value::Error));
                            }
                        };
                        let core_type = Arc::new(core_type);
                        let core_type_value = state.eval_term(&core_type);
                        core_type_entries.push((entry_name.as_ref().to_owned(), core_type));
                        state.push_param(entry_name.as_ref().to_owned(), core_type_value);
                        entry.insert(entry_name_range.clone());
                    }
                    Entry::Occupied(entry) => {
                        duplicate_names.push((
                            (*entry.key()).to_owned(),
                            entry.get().clone(),
                            entry_name_range.clone(),
                        ));
                        is_type(state, entry_type);
                    }
                }
            }

            if !duplicate_names.is_empty() {
                state.report(Message::InvalidRecordType { duplicate_names });
            }

            state.pop_many_locals(seen_names.len());
            (
                core::Term::RecordType(core_type_entries.into()),
                Arc::new(Value::Universe(max_level)),
            )
        }
        Term::RecordElim(head, name_range, name) => {
            let (core_head, head_type) = synth_type(state, head);

            match head_type.as_ref() {
                Value::RecordType(closure) => {
                    let head_value = state.eval_term(&core_head);
                    let name = name.as_ref();

                    if let Some(entry_type) = state.record_elim_type(&head_value, name, closure) {
                        let core_head = Arc::new(core_head);
                        let core_term = core::Term::RecordElim(core_head, name.to_owned());
                        return (core_term, entry_type);
                    }
                }
                Value::Error => return (core::Term::Error, Arc::new(Value::Error)),
                _ => {}
            }

            let head_type = state.read_back_type(&head_type);
            let head_type = state.delaborate_term(&head_type);
            state.report(Message::EntryNameNotFound {
                head_range: head.range(),
                name_range: name_range.clone(),
                expected_field_name: name.as_ref().to_owned(),
                head_type,
            });
            (core::Term::Error, Arc::new(Value::Error))
        }
        Term::FunctionType(param_type, body_type) => {
            match (is_type(state, param_type), is_type(state, body_type)) {
                ((core_param_type, Some(param_level)), (core_body_type, Some(body_level))) => (
                    core::Term::FunctionType(Arc::new(core_param_type), Arc::new(core_body_type)),
                    Arc::new(Value::Universe(std::cmp::max(param_level, body_level))),
                ),
                (_, _) => (core::Term::Error, Arc::new(Value::Error)),
            }
        }
        Term::FunctionTerm(_, _, _) => {
            state.report(Message::AmbiguousTerm {
                range: term.range(),
                term: AmbiguousTerm::FunctionTerm,
            });
            (core::Term::Error, Arc::new(Value::Error))
        }
        Term::FunctionElim(head, arguments) => {
            let mut head_range = head.range();
            let (mut core_head, mut head_type) = synth_type(state, head);
            let mut arguments = arguments.iter();

            while let Some(argument) = arguments.next() {
                match head_type.as_ref() {
                    Value::FunctionType(param_type, body_type) => {
                        head_range.end = argument.range().end;
                        core_head = core::Term::FunctionElim(
                            Arc::new(core_head),
                            Arc::new(check_type(state, argument, &param_type)),
                        );
                        head_type = body_type.clone();
                    }
                    Value::Error => return (core::Term::Error, Arc::new(Value::Error)),
                    _ => {
                        let head_type = state.read_back_type(&head_type);
                        let head_type = state.delaborate_term(&head_type);
                        let unexpected_arguments = arguments.map(|arg| arg.range()).collect();
                        state.report(Message::TooManyArguments {
                            head_range,
                            head_type,
                            unexpected_arguments,
                        });
                        return (core::Term::Error, Arc::new(Value::Error));
                    }
                }
            }

            (core_head, head_type)
        }
        Term::Lift(_, inner_term, offset) => {
            match state.universe_offset + core::UniverseOffset(*offset) {
                Some(new_offset) => {
                    let previous_offset = std::mem::replace(&mut state.universe_offset, new_offset);
                    let (core_term, r#type) = synth_type(state, inner_term);
                    state.universe_offset = previous_offset;
                    (core_term, r#type)
                }
                None => {
                    state.report(Message::MaximumUniverseLevelReached {
                        range: term.range(),
                    });
                    (core::Term::Error, Arc::new(Value::Error))
                }
            }
        }
        Term::Error(_) => (core::Term::Error, Arc::new(Value::Error)),
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
