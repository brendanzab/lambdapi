//! Bidirectional type checker for the core language.
//!
//! This is a simpler implementation of type checking than the one found in
//! `surface::projections::core`,
//! because it only needs to check the (much simpler) core language,
//! and doesn't need to perform any additional elaboration.
//! We can use it as a way to validate that elaborated terms are well-formed.

use std::sync::Arc;

use crate::core::semantics;
use crate::core::{
    Closure, Constant, Elim, Globals, Head, LocalLevel, Locals, Term, UniverseLevel,
    UniverseOffset, Value,
};

/// The state of the type checker.
pub struct State<'me> {
    /// Global definition environment.
    globals: &'me Globals,
    /// The current universe offset.
    universe_offset: UniverseOffset,
    /// Local type environment (used for getting the types of local variables).
    types: Locals<Arc<Value>>,
    /// Local value environment (used for evaluation).
    values: Locals<Arc<Value>>,
    /// The diagnostic messages accumulated during type checking.
    messages: Vec<Message>,
}

impl<'me> State<'me> {
    /// Construct a new type checker state.
    pub fn new(globals: &'me Globals) -> State<'me> {
        State {
            globals,
            universe_offset: UniverseOffset(0),
            types: Locals::new(),
            values: Locals::new(),
            messages: Vec::new(),
        }
    }

    /// Get the next level to be used for a local entry.
    fn next_level(&self) -> LocalLevel {
        self.values.size().next_level()
    }

    /// Push a local entry.
    fn push_local(&mut self, value: Arc<Value>, r#type: Arc<Value>) {
        self.types.push(r#type);
        self.values.push(value);
    }

    /// Push a local parameter.
    fn push_param(&mut self, r#type: Arc<Value>) -> Arc<Value> {
        let value = Arc::new(Value::local(self.next_level(), r#type.clone()));
        self.push_local(value.clone(), r#type);
        value
    }

    /// Pop a local entry.
    fn pop_local(&mut self) {
        self.types.pop();
        self.values.pop();
    }

    /// Pop the given number of local entries.
    fn pop_many_locals(&mut self, count: usize) {
        self.types.pop_many(count);
        self.values.pop_many(count);
    }

    /// Report a diagnostic message.
    fn report(&mut self, message: Message) {
        self.messages.push(message);
    }

    /// Drain the current diagnostic messages.
    pub fn drain_messages(&mut self) -> std::vec::Drain<Message> {
        self.messages.drain(..)
    }

    /// Reset the type checker state while retaining existing allocations.
    pub fn clear(&mut self) {
        self.universe_offset = UniverseOffset(0);
        self.types.clear();
        self.values.clear();
        self.messages.clear();
    }

    /// Evaluate a term using the current state of the type checker.
    pub fn eval_term(&mut self, term: &Term) -> Arc<Value> {
        semantics::eval_term(self.globals, self.universe_offset, &mut self.values, term)
    }

    /// Eliminate a closure.
    fn eval_closure_elim(&self, closure: &Closure, argument: Arc<Value>) -> Arc<Value> {
        semantics::eval_closure_elim(self.globals, closure, argument)
    }

    /// Check if `value0` is a subtype of `value1`.
    pub fn is_subtype(&self, value0: &Value, value1: &Value) -> bool {
        semantics::is_subtype(self.globals, self.values.size(), value0, value1)
    }
}

#[derive(Clone, Debug)]
pub enum Message {
    MaximumUniverseLevelReached,
    UnboundGlobal {
        name: String,
    },
    UnboundLocal,
    DuplicateNamesInRecordType {
        duplicate_names: Vec<String>,
    },
    MissingNamesInRecordTerm {
        missing_names: Vec<String>,
    },
    UnexpectedNamesInRecordTerm {
        unexpected_names: Vec<String>,
    },
    AmbiguousRecordTerm,
    FieldNotFound {
        expected_field_name: String,
        head_type: Arc<Value>,
    },
    TooManyParametersForFunctionTerm {
        expected_type: Arc<Value>,
    },
    AmbiguousFunctionTerm,
    NotAFunction {
        head_type: Arc<Value>,
    },
    AmbiguousSequence,
    MismatchedSequenceLength {
        found_len: usize,
        expected_len: Arc<Value>,
    },
    NoSequenceConversion {
        expected_type: Arc<Value>,
    },
    ExpectedType {
        found_type: Arc<Value>,
    },
    MismatchedTypes {
        found_type: Arc<Value>,
        expected_type: Arc<Value>,
    },
}

/// Check that a term is a universe and return its level.
pub fn check_type(state: &mut State<'_>, term: &Term) -> Option<UniverseLevel> {
    let r#type = synth_term(state, term);
    match r#type.as_ref() {
        Value::Universe(level) => Some(*level),
        _ => {
            state.report(Message::ExpectedType { found_type: r#type });
            None
        }
    }
}

/// Check that a term matches the expected type.
pub fn check_term(state: &mut State<'_>, term: &Term, expected_type: &Arc<Value>) {
    match (term, expected_type.as_ref()) {
        (Term::Sequence(entry_terms), Value::Elim(Head::Global(name, _), elims, _)) => {
            match (name.as_ref(), elims.as_slice()) {
                ("Array", [Elim::Function(len, _), Elim::Function(entry_type, _)]) => {
                    for entry_term in entry_terms {
                        check_term(state, entry_term, entry_type);
                    }

                    match **len {
                        Value::Constant(Constant::U32(len))
                            if len as usize == entry_terms.len() => {}
                        _ => state.report(Message::MismatchedSequenceLength {
                            found_len: entry_terms.len(),
                            expected_len: len.clone(),
                        }),
                    }
                }
                ("List", [Elim::Function(entry_type, _)]) => {
                    for entry_term in entry_terms {
                        check_term(state, entry_term, entry_type);
                    }
                }
                _ => state.report(Message::NoSequenceConversion {
                    expected_type: expected_type.clone(),
                }),
            }
        }
        (Term::Sequence(_), Value::Error) => {}
        (Term::Sequence(_), _) => state.report(Message::NoSequenceConversion {
            expected_type: expected_type.clone(),
        }),
        (Term::RecordTerm(term_entries), _) => {
            let mut missing_names = Vec::new();

            let mut expected_type = expected_type.clone();
            let mut term_entries = term_entries.clone();

            while let Value::RecordTypeExtend(entry_name, entry_type, rest_type) =
                expected_type.as_ref()
            {
                expected_type = match term_entries.remove(entry_name) {
                    Some(entry_term) => {
                        check_term(state, &entry_term, entry_type);
                        let entry_value = state.eval_term(&entry_term);
                        state.eval_closure_elim(rest_type, entry_value)
                    }
                    None => {
                        missing_names.push(entry_name.clone());
                        state.eval_closure_elim(rest_type, Arc::new(Value::Error))
                    }
                };
            }

            if !missing_names.is_empty() {
                state.report(Message::MissingNamesInRecordTerm { missing_names });
            }
            if !term_entries.is_empty() {
                let unexpected_names = (term_entries.into_iter()).map(|(name, _)| name).collect();
                state.report(Message::UnexpectedNamesInRecordTerm { unexpected_names });
            }
        }
        (Term::FunctionTerm(_, body), Value::FunctionType(param_type, body_type)) => {
            state.push_param(param_type.clone());
            check_term(state, body, body_type);
            state.pop_local();
        }
        (Term::FunctionTerm(_, _), _) => {
            state.report(Message::TooManyParametersForFunctionTerm {
                expected_type: expected_type.clone(),
            });
        }
        (term, _) => match synth_term(state, term) {
            found_type if state.is_subtype(&found_type, expected_type) => {}
            found_type => state.report(Message::MismatchedTypes {
                found_type,
                expected_type: expected_type.clone(),
            }),
        },
    }
}

/// Synthesize the type of a term.
pub fn synth_term(state: &mut State<'_>, term: &Term) -> Arc<Value> {
    match term {
        Term::Universe(level) => match *level + UniverseOffset(1) {
            Some(level) => Arc::new(Value::universe(level)),
            None => {
                state.report(Message::MaximumUniverseLevelReached);
                Arc::new(Value::Error)
            }
        },
        Term::Global(name) => match state.globals.get(name) {
            Some((r#type, _)) => state.eval_term(r#type),
            None => {
                state.report(Message::UnboundGlobal {
                    name: name.to_owned(),
                });
                Arc::new(Value::Error)
            }
        },
        Term::Local(index) => match state.types.get(*index) {
            Some(r#type) => r#type.clone(),
            None => {
                state.report(Message::UnboundLocal);
                Arc::new(Value::Error)
            }
        },
        Term::Constant(constant) => Arc::new(match constant {
            Constant::U8(_) => Value::global("U8", 0, Value::universe(0)),
            Constant::U16(_) => Value::global("U16", 0, Value::universe(0)),
            Constant::U32(_) => Value::global("U32", 0, Value::universe(0)),
            Constant::U64(_) => Value::global("U64", 0, Value::universe(0)),
            Constant::S8(_) => Value::global("S8", 0, Value::universe(0)),
            Constant::S16(_) => Value::global("S16", 0, Value::universe(0)),
            Constant::S32(_) => Value::global("S32", 0, Value::universe(0)),
            Constant::S64(_) => Value::global("S64", 0, Value::universe(0)),
            Constant::F32(_) => Value::global("F32", 0, Value::universe(0)),
            Constant::F64(_) => Value::global("F64", 0, Value::universe(0)),
            Constant::Char(_) => Value::global("Char", 0, Value::universe(0)),
            Constant::String(_) => Value::global("String", 0, Value::universe(0)),
        }),
        Term::Sequence(_) => {
            state.report(Message::AmbiguousSequence);
            Arc::new(Value::Error)
        }
        Term::Ann(term, r#type) => {
            check_type(state, r#type);
            let r#type = state.eval_term(r#type);
            check_term(state, term, &r#type);
            r#type
        }
        Term::RecordTerm(term_entries) => {
            if term_entries.is_empty() {
                Arc::from(Value::RecordTypeEmpty)
            } else {
                state.report(Message::AmbiguousRecordTerm);
                Arc::new(Value::Error)
            }
        }
        Term::RecordType(type_entries) => {
            use std::collections::BTreeSet;

            let mut max_level = UniverseLevel(0);
            let mut duplicate_names = Vec::new();
            let mut seen_names = BTreeSet::new();

            for (name, entry_type) in type_entries {
                if !seen_names.insert(name) {
                    duplicate_names.push(name.clone());
                }
                max_level = match check_type(state, entry_type) {
                    Some(level) => std::cmp::max(max_level, level),
                    None => return Arc::new(Value::Error),
                };
                let entry_type = state.eval_term(entry_type);
                state.push_param(entry_type);
            }

            state.pop_many_locals(seen_names.len());

            if !duplicate_names.is_empty() {
                state.report(Message::DuplicateNamesInRecordType { duplicate_names });
            }

            Arc::new(Value::Universe(max_level))
        }
        Term::RecordElim(head, name) => {
            let mut head_type = synth_term(state, head);

            while let Value::RecordTypeExtend(current_name, entry_type, rest_type) =
                head_type.as_ref()
            {
                if name == current_name {
                    return entry_type.clone();
                } else {
                    let value = state.eval_term(&Term::RecordElim(head.clone(), name.clone()));
                    head_type = state.eval_closure_elim(rest_type, value);
                }
            }

            state.report(Message::FieldNotFound {
                expected_field_name: name.clone(),
                head_type,
            });
            Arc::new(Value::Error)
        }
        Term::FunctionType(param_type, body_type) => {
            match (check_type(state, param_type), check_type(state, body_type)) {
                (Some(param_level), Some(body_level)) => {
                    Arc::new(Value::Universe(std::cmp::max(param_level, body_level)))
                }
                (_, _) => Arc::new(Value::Error),
            }
        }
        Term::FunctionTerm(_, _) => {
            state.report(Message::AmbiguousFunctionTerm);
            Arc::new(Value::Error)
        }
        Term::FunctionElim(head, argument) => {
            let head_type = synth_term(state, head);
            match head_type.as_ref() {
                Value::FunctionType(param_type, body_type) => {
                    check_term(state, argument, &param_type);
                    body_type.clone()
                }
                Value::Error => Arc::new(Value::Error),
                _ => {
                    state.report(Message::NotAFunction { head_type });
                    Arc::new(Value::Error)
                }
            }
        }
        Term::Lift(term, offset) => match state.universe_offset + *offset {
            Some(new_offset) => {
                let previous_offset = std::mem::replace(&mut state.universe_offset, new_offset);
                let r#type = synth_term(state, term);
                state.universe_offset = previous_offset;
                r#type
            }
            None => {
                state.report(Message::MaximumUniverseLevelReached);
                Arc::new(Value::Error)
            }
        },
        Term::Error => Arc::new(Value::Error),
    }
}
