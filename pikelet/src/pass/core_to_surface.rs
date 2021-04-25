//! Distills the [core language] into the [surface language].
//!
//! This is the inverse of [`pass::surface_to_core`], and is useful for pretty
//! printing terms when presenting them to the user.
//!
//! [surface language]: crate::lang::surface
//! [core language]: crate::lang::core
//! [`pass::surface_to_core`]: crate::pass::surface_to_core

use contracts::debug_ensures;
use fxhash::FxHashMap;

use crate::lang::core::{Constant, EnvSize, Globals, Term, TermData, VarIndex};
use crate::lang::surface;
use crate::lang::Located;

/// Distillation context.
pub struct Context<'globals> {
    globals: &'globals Globals,
    usages: FxHashMap<String, Usage>,
    names: Vec<String>,
}

struct Usage {
    base_name: Option<String>,
    count: usize,
}

impl Usage {
    fn new() -> Usage {
        Usage {
            base_name: None,
            count: 1,
        }
    }
}

const DEFAULT_NAME: &str = "t";

impl<'globals> Context<'globals> {
    /// Construct a new distillation state.
    pub fn new(globals: &'globals Globals) -> Context<'globals> {
        let usages = globals
            .entries()
            .map(|(name, _)| (name.to_owned(), Usage::new()))
            .collect();

        Context {
            globals,
            usages,
            names: Vec::new(),
        }
    }

    fn index_to_level(&self, index: VarIndex) -> usize {
        let index = index.to_usize();
        self.names.len().saturating_sub(index).saturating_sub(1)
    }

    fn get_name(&self, index: VarIndex) -> Option<&str> {
        Some(self.names.get(self.index_to_level(index))?.as_str())
    }

    // FIXME: This is incredibly horrific and I do not like it!
    //
    // We could investigate finding more optimal optimal names by using free
    // variables, or look into [scope sets](https://typesanitizer.com/blog/scope-sets-as-pinata.html)
    // for a more principled approach to scope names.
    pub fn push_scope(&mut self, name_hint: Option<&str>) -> String {
        let base_name = name_hint.unwrap_or(DEFAULT_NAME);
        let (fresh_name, base_name) = match self.usages.get_mut(base_name) {
            // The name has not been used yet
            None => (base_name.to_owned(), None),
            // The name is in use - find a free one to use!
            Some(usage) => {
                let mut suffix = usage.count;
                // Update the usage count to make finding the next name faster.
                usage.count += 1;
                // Attempt names with incrementing numeric suffixes until we
                // find one that has yet to be used.
                loop {
                    // TODO: Reduce string allocations
                    match format!("{}-{}", base_name, suffix) {
                        // Candidate name has been used - try another!
                        name if self.usages.contains_key(&name) => suffix += 1,
                        // The candidate has not been used - we're free to use it
                        name => break (name, Some(base_name.to_owned())),
                    }
                }
            }
        };

        let usage = Usage {
            base_name,
            count: 1,
        };
        // TODO: Reduce cloning of names
        self.usages.insert(fresh_name.clone(), usage);
        self.names.push(fresh_name.clone());
        fresh_name
    }

    pub fn pop_scope(&mut self) {
        if let Some(mut name) = self.names.pop() {
            while let Some(base_name) = self.remove_usage(name) {
                name = base_name;
            }
        }
    }

    pub fn pop_scopes(&mut self, count: usize) {
        (0..count).for_each(|_| self.pop_scope());
    }

    pub fn truncate_scopes(&mut self, count: EnvSize) {
        (count.to_usize()..self.names.len()).for_each(|_| self.pop_scope());
    }

    fn remove_usage(&mut self, name: String) -> Option<String> {
        use std::collections::hash_map::Entry;

        match self.usages.entry(name) {
            Entry::Occupied(entry) if entry.get().count >= 1 => entry.remove().base_name,
            Entry::Occupied(mut entry) => {
                entry.get_mut().count -= 1;
                None
            }
            Entry::Vacant(_) => None,
        }
    }

    /// Distill a [`core::Term`] into a [`surface::Term`].
    ///
    /// [`core::Term`]: crate::lang::core::Term
    /// [`surface::Term`]: crate::lang::surface::Term
    #[debug_ensures(self.names.len() == old(self.names.len()))]
    pub fn from_term(&mut self, term: &Term) -> surface::Term {
        let term_data = match &term.data {
            TermData::Global(name) => match self.globals.get(name) {
                Some(_) => surface::TermData::Name(name.to_owned()),
                None => surface::TermData::Error, // TODO: Log error?
            },
            TermData::Var(index) => match self.get_name(*index) {
                Some(name) => surface::TermData::Name(name.to_owned()),
                None => surface::TermData::Error, // TODO: Log error?
            },

            TermData::Ann(term, r#type) => surface::TermData::Ann(
                Box::new(self.from_term(term)),
                Box::new(self.from_term(r#type)),
            ),

            TermData::TypeType => surface::TermData::Name("Type".to_owned()),

            TermData::FunctionType(input_name_hint, input_type, output_type) => {
                // FIXME: properly group inputs!
                let input_type = self.from_term(input_type);
                let fresh_input_name =
                    self.push_scope(input_name_hint.as_ref().map(String::as_str));
                let input_type_groups =
                    vec![(vec![Located::generated(fresh_input_name)], input_type)];
                let output_type = self.from_term(output_type);
                self.pop_scopes(input_type_groups.iter().map(|(ns, _)| ns.len()).sum());

                surface::TermData::FunctionType(input_type_groups, Box::new(output_type))
            }
            TermData::FunctionTerm(input_name_hint, output_term) => {
                let mut current_output_term = output_term;

                let fresh_input_name = self.push_scope(Some(input_name_hint));
                let mut input_names = vec![Located::generated(fresh_input_name)];

                while let TermData::FunctionTerm(input_name_hint, output_term) =
                    &current_output_term.data
                {
                    let fresh_input_name = self.push_scope(Some(input_name_hint));
                    input_names.push(Located::generated(fresh_input_name));
                    current_output_term = output_term;
                }

                let output_term = self.from_term(current_output_term);
                self.pop_scopes(input_names.len());

                surface::TermData::FunctionTerm(input_names, Box::new(output_term))
            }
            TermData::FunctionElim(head_term, input_term) => {
                let mut current_head_term = head_term;

                let mut input_terms = vec![self.from_term(input_term)];
                while let TermData::FunctionElim(head_term, input_term) = &current_head_term.data {
                    input_terms.push(self.from_term(input_term));
                    current_head_term = head_term;
                }
                input_terms.reverse();

                let head_term = self.from_term(current_head_term);
                surface::TermData::FunctionElim(Box::new(head_term), input_terms)
            }

            TermData::RecordType(labels, types) => {
                let type_entries = Iterator::zip(labels.iter(), types.iter())
                    .map(|(label, entry_type)| {
                        let entry_type = self.from_term(entry_type);
                        let label = label.clone();
                        match self.push_scope(Some(&label)) {
                            name if name == label => (Located::generated(label), None, entry_type),
                            name => (
                                Located::generated(label),
                                Some(Located::generated(name)),
                                entry_type,
                            ),
                        }
                    })
                    .collect::<Vec<_>>();
                self.pop_scopes(type_entries.len());

                surface::TermData::RecordType(type_entries)
            }
            TermData::RecordTerm(labels, terms) => {
                let term_entries = Iterator::zip(labels.iter(), terms.iter())
                    .map(|(label, entry_type)| {
                        let entry_type = self.from_term(entry_type);
                        let label = label.clone();
                        match self.push_scope(Some(&label)) {
                            name if name == label => (Located::generated(label), None, entry_type),
                            name => (
                                Located::generated(label),
                                Some(Located::generated(name)),
                                entry_type,
                            ),
                        }
                    })
                    .collect::<Vec<_>>();
                self.pop_scopes(term_entries.len());

                surface::TermData::RecordTerm(term_entries)
            }
            TermData::RecordElim(head_term, label) => surface::TermData::RecordElim(
                Box::new(self.from_term(head_term)),
                Located::generated(label.clone()),
            ),

            TermData::ArrayTerm(entry_terms) | TermData::ListTerm(entry_terms) => {
                let core_entry_terms = entry_terms
                    .iter()
                    .map(|entry_term| self.from_term(entry_term))
                    .collect();

                surface::TermData::SequenceTerm(core_entry_terms)
            }

            TermData::Constant(constant) => match constant {
                Constant::U8(value) => surface::TermData::NumberTerm(value.to_string()),
                Constant::U16(value) => surface::TermData::NumberTerm(value.to_string()),
                Constant::U32(value) => surface::TermData::NumberTerm(value.to_string()),
                Constant::U64(value) => surface::TermData::NumberTerm(value.to_string()),
                Constant::S8(value) => surface::TermData::NumberTerm(value.to_string()),
                Constant::S16(value) => surface::TermData::NumberTerm(value.to_string()),
                Constant::S32(value) => surface::TermData::NumberTerm(value.to_string()),
                Constant::S64(value) => surface::TermData::NumberTerm(value.to_string()),
                Constant::F32(value) => surface::TermData::NumberTerm(value.to_string()),
                Constant::F64(value) => surface::TermData::NumberTerm(value.to_string()),
                Constant::Char(value) => surface::TermData::CharTerm(format!("{:?}", value)),
                Constant::String(value) => surface::TermData::StringTerm(format!("{:?}", value)),
            },

            TermData::Error => surface::TermData::Error,
        };

        surface::Term::generated(term_data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_default_name() {
        let globals = Globals::default();
        let mut state = Context::new(&globals);

        assert_eq!(state.push_scope(None), "t");
        assert_eq!(state.push_scope(Some("t")), "t-1");
        assert_eq!(state.push_scope(None), "t-2");
    }

    #[test]
    fn push_and_pop_default_name() {
        let globals = Globals::default();
        let mut state = Context::new(&globals);

        assert_eq!(state.push_scope(None), "t");
        state.pop_scope();
        assert_eq!(state.push_scope(None), "t");
        assert_eq!(state.push_scope(None), "t-1");
        state.pop_scope();
        state.pop_scope();
        assert_eq!(state.push_scope(None), "t");
        assert_eq!(state.push_scope(None), "t-1");
        assert_eq!(state.push_scope(None), "t-2");
        state.pop_scope();
        state.pop_scope();
        state.pop_scope();
        assert_eq!(state.push_scope(None), "t");
        assert_eq!(state.push_scope(None), "t-1");
        assert_eq!(state.push_scope(None), "t-2");
    }

    #[test]
    fn push_scope() {
        let globals = Globals::default();
        let mut state = Context::new(&globals);

        assert_eq!(state.push_scope(Some("test")), "test");
        assert_eq!(state.push_scope(Some("test")), "test-1");
        assert_eq!(state.push_scope(Some("test")), "test-2");
    }

    #[test]
    fn push_and_pop_scope() {
        let globals = Globals::default();
        let mut state = Context::new(&globals);

        assert_eq!(state.push_scope(Some("test")), "test");
        state.pop_scope();
        assert_eq!(state.push_scope(Some("test")), "test");
        assert_eq!(state.push_scope(Some("test")), "test-1");
        state.pop_scope();
        state.pop_scope();
        assert_eq!(state.push_scope(Some("test")), "test");
        assert_eq!(state.push_scope(Some("test")), "test-1");
        assert_eq!(state.push_scope(Some("test")), "test-2");
        state.pop_scope();
        state.pop_scope();
        state.pop_scope();
        assert_eq!(state.push_scope(Some("test")), "test");
        assert_eq!(state.push_scope(Some("test")), "test-1");
        assert_eq!(state.push_scope(Some("test")), "test-2");
    }

    #[test]
    fn push_fresh_name() {
        let globals = Globals::default();
        let mut state = Context::new(&globals);

        assert_eq!(state.push_scope(Some("test")), "test");
        assert_eq!(state.push_scope(Some("test")), "test-1");
        assert_eq!(state.push_scope(Some("test-1")), "test-1-1");
        assert_eq!(state.push_scope(Some("test-1")), "test-1-2");
        assert_eq!(state.push_scope(Some("test-1-2")), "test-1-2-1");
    }

    #[test]
    fn push_global_name() {
        let globals = Globals::default();
        let mut state = Context::new(&globals);

        assert_eq!(state.push_scope(Some("Type")), "Type-1");
        assert_eq!(state.push_scope(Some("Type")), "Type-2");
    }
}
