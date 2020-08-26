//! The operational semantics of the language.

use std::collections::BTreeMap;
use std::sync::Arc;

use crate::lang::core::{
    Constant, Globals, LocalLevel, LocalSize, Locals, Term, UniverseLevel, UniverseOffset,
};

/// Values in the core language.
#[derive(Clone, Debug)]
pub enum Value {
    /// The type of types.
    Universe(UniverseLevel),
    /// A suspended elimination (neutral value).
    ///
    /// This is a value that cannot be reduced further as a result of being
    /// stuck on some head. Instead we maintain a 'spine' of eliminators so that
    /// we may perform further reduction later on.
    Elim(Head, Vec<Elim>),
    /// Constants.
    Constant(Constant),
    /// Ordered sequences.
    Sequence(Vec<Arc<Value>>),
    /// Record types.
    RecordType(RecordTypeClosure),
    /// Record terms.
    RecordTerm(BTreeMap<String, Arc<Value>>),
    /// Function types.
    FunctionType(Option<String>, Arc<Value>, Closure),
    /// Function terms (lambda abstractions).
    FunctionTerm(String, Closure),
    /// Error sentinel.
    Error,
}

impl Value {
    /// Create a universe at the given level.
    pub fn universe(level: impl Into<UniverseLevel>) -> Value {
        Value::Universe(level.into())
    }

    /// Create a global variable.
    pub fn global(name: impl Into<String>, offset: impl Into<UniverseOffset>) -> Value {
        Value::Elim(Head::Global(name.into(), offset.into()), Vec::new())
    }

    /// Create a local variable.
    pub fn local(level: impl Into<LocalLevel>) -> Value {
        Value::Elim(Head::Local(level.into()), Vec::new())
    }
}

/// The head of an elimination.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Head {
    /// Global variables.
    Global(String, UniverseOffset),
    /// Local variables.
    Local(LocalLevel),
}

/// An eliminator, to be used in the spine of an elimination.
#[derive(Clone, Debug)]
pub enum Elim {
    /// Record eliminators (field access).
    Record(String),
    /// Function eliminatiors (function application).
    Function(Arc<Value>),
}

/// Closure, capturing the current universe offset and the current locals in scope.
#[derive(Clone, Debug)]
pub struct Closure {
    universe_offset: UniverseOffset,
    values: Locals<Arc<Value>>,
    term: Arc<Term>,
}

impl Closure {
    pub fn new(
        universe_offset: UniverseOffset,
        values: Locals<Arc<Value>>,
        term: Arc<Term>,
    ) -> Closure {
        Closure {
            universe_offset,
            values,
            term,
        }
    }

    /// Eliminate a closure.
    pub fn elim(&self, globals: &Globals, argument: Arc<Value>) -> Arc<Value> {
        let mut values = self.values.clone();
        values.push(argument);
        eval_term(globals, self.universe_offset, &mut values, &self.term)
    }
}

/// Record type closure, capturing the current universe offset and the current locals in scope.
#[derive(Clone, Debug)]
pub struct RecordTypeClosure {
    universe_offset: UniverseOffset,
    values: Locals<Arc<Value>>,
    entries: Arc<[(String, Arc<Term>)]>,
}

impl RecordTypeClosure {
    pub fn new(
        universe_offset: UniverseOffset,
        values: Locals<Arc<Value>>,
        entries: Arc<[(String, Arc<Term>)]>,
    ) -> RecordTypeClosure {
        RecordTypeClosure {
            universe_offset,
            values,
            entries,
        }
    }

    /// Apply a callback to each of the entry types in the record closure.
    pub fn entries<'closure>(
        &'closure self,
        globals: &Globals,
        mut on_entry: impl FnMut(&'closure str, Arc<Value>) -> Arc<Value>,
    ) {
        let universe_offset = self.universe_offset;
        let mut values = self.values.clone();
        for (label, entry_type) in self.entries.iter() {
            let entry_type = eval_term(globals, universe_offset, &mut values, entry_type);
            values.push(on_entry(label, entry_type));
        }
    }
}

/// Fully normalize a term.
pub fn normalize_term(
    globals: &Globals,
    universe_offset: UniverseOffset,
    values: &mut Locals<Arc<Value>>,
    term: &Term,
) -> Term {
    read_back_value(
        globals,
        values.size(),
        &eval_term(globals, universe_offset, values, term),
    )
}

/// Evaluate a term into a value in weak-head normal form.
pub fn eval_term(
    globals: &Globals,
    universe_offset: UniverseOffset,
    values: &mut Locals<Arc<Value>>,
    term: &Term,
) -> Arc<Value> {
    match term {
        Term::Universe(level) => Arc::new(Value::universe(
            (*level + universe_offset).unwrap(), // FIXME: Handle overflow
        )),
        Term::Global(name) => match globals.get(name) {
            Some((_, Some(term))) => eval_term(globals, universe_offset, values, term),
            Some((_, None)) => Arc::new(Value::global(name, universe_offset)),
            None => Arc::new(Value::Error),
        },
        Term::Local(index) => match values.get(*index) {
            Some(value) => value.clone(),
            None => Arc::new(Value::Error),
        },
        Term::Constant(constant) => Arc::new(Value::Constant(constant.clone())),
        Term::Sequence(term_entries) => {
            let value_entries = term_entries
                .iter()
                .map(|entry_term| eval_term(globals, universe_offset, values, entry_term))
                .collect();

            Arc::new(Value::Sequence(value_entries))
        }
        Term::Ann(term, _) => eval_term(globals, universe_offset, values, term),
        Term::RecordType(type_entries) => Arc::new(Value::RecordType(RecordTypeClosure::new(
            universe_offset,
            values.clone(),
            type_entries.clone(),
        ))),
        Term::RecordTerm(term_entries) => {
            let value_entries = term_entries
                .iter()
                .map(|(label, entry_term)| {
                    let entry_term = eval_term(globals, universe_offset, values, entry_term);
                    (label.clone(), entry_term)
                })
                .collect();

            Arc::new(Value::RecordTerm(value_entries))
        }
        Term::RecordElim(head, label) => {
            let head = eval_term(globals, universe_offset, values, head);
            eval_record_elim(&head, label)
        }
        Term::FunctionType(param_name_hint, param_type, body_type) => {
            let param_type = eval_term(globals, universe_offset, values, param_type);
            let body_type = Closure::new(universe_offset, values.clone(), body_type.clone());

            Arc::new(Value::FunctionType(
                param_name_hint.clone(),
                param_type,
                body_type,
            ))
        }
        Term::FunctionTerm(param_name, body) => Arc::new(Value::FunctionTerm(
            param_name.clone(),
            Closure::new(universe_offset, values.clone(), body.clone()),
        )),
        Term::FunctionElim(head, argument) => {
            let head = eval_term(globals, universe_offset, values, head);
            let argument = eval_term(globals, universe_offset, values, argument);
            eval_fun_elim(globals, &head, argument)
        }
        Term::Lift(term, offset) => {
            let universe_offset = (universe_offset + *offset).unwrap(); // FIXME: Handle overflow
            eval_term(globals, universe_offset, values, term)
        }
        Term::Error => Arc::new(Value::Error),
    }
}

/// Return the type of the record elimination.
pub fn record_elim_type(
    globals: &Globals,
    head_value: &Value,
    label: &str,
    closure: &RecordTypeClosure,
) -> Option<Arc<Value>> {
    let universe_offset = closure.universe_offset;
    let mut values = closure.values.clone();
    for (entry_label, entry_type) in closure.entries.iter() {
        if entry_label == label {
            return Some(eval_term(globals, universe_offset, &mut values, entry_type));
        }
        values.push(eval_record_elim(head_value, label));
    }
    None
}

/// Eliminate a record term.
pub fn eval_record_elim(head_value: &Value, label: &str) -> Arc<Value> {
    match head_value {
        Value::RecordTerm(term_entries) => match term_entries.get(label) {
            Some(value) => value.clone(),
            None => Arc::new(Value::Error),
        },
        Value::Elim(head, elims) => {
            let mut elims = elims.clone(); // FIXME: Avoid clone of elims?
            elims.push(Elim::Record(label.to_owned()));
            Arc::new(Value::Elim(head.clone(), elims))
        }
        _ => Arc::new(Value::Error),
    }
}

/// Eliminate a function term.
pub fn eval_fun_elim(globals: &Globals, head_value: &Value, argument: Arc<Value>) -> Arc<Value> {
    match head_value {
        Value::FunctionTerm(_, body_closure) => body_closure.elim(globals, argument),
        Value::Elim(head, elims) => {
            let mut elims = elims.clone(); // FIXME: Avoid clone of elims?
            elims.push(Elim::Function(argument));
            Arc::new(Value::Elim(head.clone(), elims))
        }
        _ => Arc::new(Value::Error),
    }
}

/// Read-back an eliminator into the term syntax.
pub fn read_back_elim(
    globals: &Globals,
    local_size: LocalSize,
    head: &Head,
    spine: &[Elim],
) -> Term {
    let head = match head {
        Head::Global(name, shift) => Term::Global(name.clone()).lift(*shift),
        Head::Local(level) => Term::Local(local_size.index(*level)), // TODO: error
    };

    spine.iter().fold(head, |head, elim| match elim {
        Elim::Record(label) => Term::RecordElim(Arc::new(head), label.clone()),
        Elim::Function(argument) => Term::FunctionElim(
            Arc::new(head),
            Arc::new(read_back_value(globals, local_size, argument)),
        ),
    })
}

/// Read-back a value into the term syntax.
pub fn read_back_value(globals: &Globals, local_size: LocalSize, value: &Value) -> Term {
    match value {
        Value::Universe(level) => Term::Universe(*level),
        Value::Elim(head, spine) => read_back_elim(globals, local_size, head, spine),
        Value::Constant(constant) => Term::Constant(constant.clone()),
        Value::Sequence(value_entries) => {
            let term_entries = value_entries
                .iter()
                .map(|value_entry| Arc::new(read_back_value(globals, local_size, value_entry)))
                .collect();

            Term::Sequence(term_entries)
        }
        Value::RecordType(closure) => {
            let mut local_size = local_size;
            let mut type_entries = Vec::with_capacity(closure.entries.len());

            closure.entries(globals, |label, entry_type| {
                type_entries.push((
                    label.to_owned(),
                    Arc::new(read_back_value(globals, local_size, &entry_type)),
                ));

                let local_level = local_size.next_level();
                local_size = local_size.increment();

                Arc::new(Value::local(local_level))
            });

            Term::RecordType(type_entries.into())
        }
        Value::RecordTerm(value_entries) => {
            let term_entries = value_entries
                .iter()
                .map(|(label, entry_value)| {
                    (
                        label.to_owned(),
                        Arc::new(read_back_value(globals, local_size, &entry_value)),
                    )
                })
                .collect();

            Term::RecordTerm(term_entries)
        }
        Value::FunctionType(param_name_hint, param_type, body_closure) => {
            let local = Arc::new(Value::local(local_size.next_level()));
            let param_type = Arc::new(read_back_value(globals, local_size, param_type));
            let body_type = Arc::new(read_back_value(
                globals,
                local_size.increment(),
                &*body_closure.elim(globals, local),
            ));

            Term::FunctionType(param_name_hint.clone(), param_type, body_type)
        }
        Value::FunctionTerm(param_name_hint, body_closure) => {
            let local = Arc::new(Value::local(local_size.next_level()));
            let body = read_back_value(
                globals,
                LocalSize(local_size.0 + 1),
                &body_closure.elim(globals, local),
            );

            Term::FunctionTerm(param_name_hint.clone(), Arc::new(body))
        }
        Value::Error => Term::Error,
    }
}

/// Check that one elimination is equal to another elimination.
pub fn is_equal_elim(
    globals: &Globals,
    local_size: LocalSize,
    (head0, spine0): (&Head, &[Elim]),
    (head1, spine1): (&Head, &[Elim]),
) -> bool {
    head0 == head1
        && spine0.len() == spine1.len()
        && Iterator::zip(spine0.iter(), spine1.iter()).all(|(elim0, elim1)| match (elim0, elim1) {
            (Elim::Function(argument0), Elim::Function(argument1)) => {
                is_equal_nf(globals, local_size, argument0, argument1)
            }
            (Elim::Record(label0), Elim::Record(label1)) => label0 == label1,
            (_, _) => false,
        })
}

/// Check that one normal form is a equal of another normal form.
pub fn is_equal_nf(
    globals: &Globals,
    local_size: LocalSize,
    value0: &Value,
    value1: &Value,
) -> bool {
    // TODO: avoid allocation of intermediate term, as in smalltt and blott,
    // for example, see: https://github.com/jozefg/blott/blob/9eadd6f1eb3ecb28fd66a25bc56c19041d98f722/src/lib/nbe.ml#L200-L242
    read_back_value(globals, local_size, value0) == read_back_value(globals, local_size, value1)
}

/// Compare two types.
fn compare_types(
    globals: &Globals,
    local_size: LocalSize,
    value0: &Value,
    value1: &Value,
    compare: &impl Fn(UniverseLevel, UniverseLevel) -> bool,
) -> bool {
    match (value0, value1) {
        (Value::Universe(level0), Value::Universe(level1)) => compare(*level0, *level1),
        (Value::Elim(head0, spine0), Value::Elim(head1, spine1)) => {
            is_equal_elim(globals, local_size, (head0, spine0), (head1, spine1))
        }
        (Value::RecordType(closure0), Value::RecordType(closure1)) => {
            closure0.entries.len() == closure1.entries.len() && {
                let mut local_size = local_size;
                let universe_offset0 = closure0.universe_offset;
                let universe_offset1 = closure1.universe_offset;
                let mut values0 = closure0.values.clone();
                let mut values1 = closure1.values.clone();

                Iterator::zip(closure0.entries.iter(), closure1.entries.iter()).all(
                    |((label0, entry_type0), (label1, entry_type1))| {
                        label0 == label1 && {
                            let cmp = compare_types(
                                globals,
                                local_size,
                                &eval_term(globals, universe_offset0, &mut values0, entry_type0),
                                &eval_term(globals, universe_offset1, &mut values1, entry_type1),
                                compare,
                            );

                            let local_level = local_size.next_level();
                            values0.push(Arc::new(Value::local(local_level)));
                            values1.push(Arc::new(Value::local(local_level)));
                            local_size = local_size.increment();

                            cmp
                        }
                    },
                )
            }
        }
        (
            Value::FunctionType(_, param_type0, body_closure0),
            Value::FunctionType(_, param_type1, body_closure1),
        ) => {
            compare_types(globals, local_size, param_type1, param_type0, compare) && {
                let local = Arc::new(Value::local(local_size.next_level()));

                compare_types(
                    globals,
                    local_size.increment(),
                    &*body_closure0.elim(globals, local.clone()),
                    &*body_closure1.elim(globals, local),
                    compare,
                )
            }
        }
        // Errors are always treated as subtypes, regardless of what they are compared with.
        (Value::Error, _) | (_, Value::Error) => true,
        // Anything else is not equal!
        (_, _) => false,
    }
}

/// Check that one type is a equal to another type.
pub fn is_equal_type(
    globals: &Globals,
    local_size: LocalSize,
    value0: &Value,
    value1: &Value,
) -> bool {
    compare_types(globals, local_size, value0, value1, &|l0, l1| l0 == l1)
}

/// Check that one type is a subtype of another type.
pub fn is_subtype(
    globals: &Globals,
    local_size: LocalSize,
    value0: &Value,
    value1: &Value,
) -> bool {
    compare_types(globals, local_size, value0, value1, &|l0, l1| l0 <= l1)
}
