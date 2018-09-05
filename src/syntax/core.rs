//! The core syntax of the language

use moniker::{Binder, Embed, FreeVar, Nest, Scope, Var};
use std::fmt;
use std::ops;
use std::rc::Rc;

use syntax::pretty::{self, ToDoc};
use syntax::{Label, Level};

/// A module definition
pub struct Module {
    /// The items contained in the module
    pub items: Vec<Item>,
}

/// Top-level items within a module
#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    /// Declares the type associated with a label, prior to its definition
    Declaration {
        /// The external name for this declaration, to be used when referring
        /// to this item from other modules
        label: Label,
        /// The internal name for this declaration., to be used when binding
        /// this name to variables
        binder: Binder<String>,
        /// The type annotation for associated with the label
        term: RcTerm,
    },
    /// Defines the term that should be associated with a label
    Definition {
        /// The external name for this definition, to be used when referring
        /// to this item from other modules
        label: Label,
        /// The internal name for this definition., to be used when binding
        /// this name to variables
        binder: Binder<String>,
        /// The term for associated with the label
        term: RcTerm,
    },
}

/// Literals
///
/// We could church encode all the things, but that would be prohibitively expensive!
#[derive(Debug, Clone, PartialEq, PartialOrd, BoundTerm, BoundPattern)]
pub enum Literal {
    Bool(bool),
    String(String),
    Char(char),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
}

impl fmt::Display for Literal {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.to_doc().group().render_fmt(pretty::FALLBACK_WIDTH, f)
    }
}

#[derive(Debug, Clone, PartialEq, BoundPattern)]
pub enum Pattern {
    /// Patterns annotated with types
    Ann(RcPattern, Embed<RcTerm>),
    /// Patterns that bind variables
    Binder(Binder<String>),
    /// Literal patterns
    Literal(Literal),
}

impl fmt::Display for Pattern {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.to_doc().group().render_fmt(pretty::FALLBACK_WIDTH, f)
    }
}

/// Reference counted patterns
#[derive(Debug, Clone, PartialEq, BoundPattern)]
pub struct RcPattern {
    pub inner: Rc<Pattern>,
}

impl From<Pattern> for RcPattern {
    fn from(src: Pattern) -> RcPattern {
        RcPattern {
            inner: Rc::new(src),
        }
    }
}

impl ops::Deref for RcPattern {
    type Target = Pattern;

    fn deref(&self) -> &Pattern {
        &self.inner
    }
}

impl fmt::Display for RcPattern {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.inner, f)
    }
}

/// The core term syntax
#[derive(Debug, Clone, PartialEq, BoundTerm)]
pub enum Term {
    /// A term annotated with a type
    Ann(RcTerm, RcTerm),
    /// Universes
    Universe(Level),
    /// Literals
    Literal(Literal),
    /// A variable
    Var(Var<String>),
    /// An external definition
    Extern(String, RcTerm),
    /// A global name
    Global(String),
    /// Dependent function types
    Pi(Scope<(Binder<String>, Embed<RcTerm>), RcTerm>),
    /// Lambda abstractions
    Lam(Scope<(Binder<String>, Embed<RcTerm>), RcTerm>),
    /// Term application
    App(RcTerm, RcTerm),
    /// Dependent record types
    RecordType(Scope<Nest<(Label, Binder<String>, Embed<RcTerm>)>, ()>),
    /// Dependent record
    Record(Scope<Nest<(Label, Binder<String>, Embed<RcTerm>)>, ()>),
    /// Field projection
    Proj(RcTerm, Label),
    /// Case expressions
    Case(RcTerm, Vec<Scope<RcPattern, RcTerm>>),
    /// Array literals
    Array(Vec<RcTerm>),
}

impl Term {
    pub fn universe(level: impl Into<Level>) -> Term {
        Term::Universe(level.into())
    }

    pub fn global(name: impl Into<String>) -> Term {
        Term::Global(name.into())
    }
}

impl fmt::Display for Term {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.to_doc().group().render_fmt(pretty::FALLBACK_WIDTH, f)
    }
}

/// Reference counted terms
#[derive(Debug, Clone, PartialEq, BoundTerm)]
pub struct RcTerm {
    pub inner: Rc<Term>,
}

impl RcTerm {
    pub fn substs(&self, mappings: &[(FreeVar<String>, RcTerm)]) -> RcTerm {
        match *self.inner {
            Term::Ann(ref term, ref ty) => {
                RcTerm::from(Term::Ann(term.substs(mappings), ty.substs(mappings)))
            },
            Term::Universe(_) | Term::Literal(_) | Term::Global(_) => self.clone(),
            Term::Var(ref var) => match mappings.iter().find(|&(ref name, _)| var == name) {
                Some(&(_, ref term)) => term.clone(),
                None => self.clone(),
            },
            Term::Extern(ref name, ref ty) => {
                RcTerm::from(Term::Extern(name.clone(), ty.substs(mappings)))
            },
            Term::Pi(ref scope) => {
                let (ref name, Embed(ref ann)) = scope.unsafe_pattern;
                RcTerm::from(Term::Pi(Scope {
                    unsafe_pattern: (name.clone(), Embed(ann.substs(mappings))),
                    unsafe_body: scope.unsafe_body.substs(mappings),
                }))
            },
            Term::Lam(ref scope) => {
                let (ref name, Embed(ref ann)) = scope.unsafe_pattern;
                RcTerm::from(Term::Lam(Scope {
                    unsafe_pattern: (name.clone(), Embed(ann.substs(mappings))),
                    unsafe_body: scope.unsafe_body.substs(mappings),
                }))
            },
            Term::App(ref head, ref arg) => {
                RcTerm::from(Term::App(head.substs(mappings), arg.substs(mappings)))
            },
            Term::RecordType(ref scope) | Term::Record(ref scope)
                if scope.unsafe_pattern.unsafe_patterns.is_empty() =>
            {
                self.clone()
            },
            Term::RecordType(ref scope) => {
                let unsafe_patterns = scope
                    .unsafe_pattern
                    .unsafe_patterns
                    .iter()
                    .map(|&(ref label, ref binder, Embed(ref ann))| {
                        (label.clone(), binder.clone(), Embed(ann.substs(mappings)))
                    }).collect();

                RcTerm::from(Term::RecordType(Scope {
                    unsafe_pattern: Nest { unsafe_patterns },
                    unsafe_body: (),
                }))
            },
            Term::Record(ref scope) => {
                let unsafe_patterns = scope
                    .unsafe_pattern
                    .unsafe_patterns
                    .iter()
                    .map(|&(ref label, ref binder, Embed(ref expr))| {
                        (label.clone(), binder.clone(), Embed(expr.substs(mappings)))
                    }).collect();

                RcTerm::from(Term::Record(Scope {
                    unsafe_pattern: Nest { unsafe_patterns },
                    unsafe_body: (),
                }))
            },
            Term::Proj(ref expr, ref label) => {
                RcTerm::from(Term::Proj(expr.substs(mappings), label.clone()))
            },
            Term::Case(ref head, ref clauses) => RcTerm::from(Term::Case(
                head.substs(mappings),
                clauses
                    .iter()
                    .map(|scope| Scope {
                        unsafe_pattern: scope.unsafe_pattern.clone(), // subst?
                        unsafe_body: scope.unsafe_body.substs(mappings),
                    }).collect(),
            )),
            Term::Array(ref elems) => RcTerm::from(Term::Array(
                elems.iter().map(|elem| elem.substs(mappings)).collect(),
            )),
        }
    }
}

impl From<Term> for RcTerm {
    fn from(src: Term) -> RcTerm {
        RcTerm {
            inner: Rc::new(src),
        }
    }
}

impl ops::Deref for RcTerm {
    type Target = Term;

    fn deref(&self) -> &Term {
        &self.inner
    }
}

impl fmt::Display for RcTerm {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.inner, f)
    }
}

/// Values
///
/// These are either in _normal form_ (they cannot be reduced further) or are
/// _neutral terms_ (there is a possibility of reducing further depending
/// on the bindings given in the context)
#[derive(Debug, Clone, PartialEq, BoundTerm)]
pub enum Value {
    /// Universes
    Universe(Level),
    /// Literals
    Literal(Literal),
    /// A pi type
    Pi(Scope<(Binder<String>, Embed<RcValue>), RcValue>),
    /// A lambda abstraction
    Lam(Scope<(Binder<String>, Embed<RcValue>), RcValue>),
    /// Dependent record types
    RecordType(Scope<Nest<(Label, Binder<String>, Embed<RcValue>)>, ()>),
    /// Dependent record
    Record(Scope<Nest<(Label, Binder<String>, Embed<RcValue>)>, ()>),
    /// Array literals
    Array(Vec<RcValue>),
    /// Neutral terms
    ///
    /// A term whose computation has stopped because of an attempt to compute an
    /// application `Head`.
    Neutral(RcNeutral, Spine),
}

impl Value {
    pub fn universe(level: impl Into<Level>) -> Value {
        Value::Universe(level.into())
    }

    pub fn global(name: impl Into<String>) -> Value {
        Value::Neutral(RcNeutral::from(Neutral::global(name)), Spine::new())
    }

    pub fn substs(&self, mappings: &[(FreeVar<String>, RcTerm)]) -> RcTerm {
        // FIXME: This seems quite wasteful!
        RcTerm::from(Term::from(self)).substs(mappings)
    }

    /// Returns `true` if the value is in weak head normal form
    pub fn is_whnf(&self) -> bool {
        match *self {
            Value::Universe(_)
            | Value::Literal(_)
            | Value::Pi(_)
            | Value::Lam(_)
            | Value::RecordType(_)
            | Value::Record(_)
            | Value::Array(_) => true,
            Value::Neutral(_, _) => false,
        }
    }

    /// Returns `true` if the value is in normal form (ie. it contains no neutral terms within it)
    pub fn is_nf(&self) -> bool {
        match *self {
            Value::Universe(_) | Value::Literal(_) => true,
            Value::Pi(ref scope) | Value::Lam(ref scope) => {
                (scope.unsafe_pattern.1).0.is_nf() && scope.unsafe_body.is_nf()
            },
            Value::RecordType(ref scope) | Value::Record(ref scope) => scope
                .unsafe_pattern
                .unsafe_patterns
                .iter()
                .all(|(_, _, Embed(ref term))| term.is_nf()),
            Value::Array(ref elems) => elems.iter().all(|elem| elem.is_nf()),
            Value::Neutral(_, _) => false,
        }
    }

    pub fn head_app(&self) -> Option<(&Head, &Spine)> {
        if let Value::Neutral(ref neutral, ref spine) = *self {
            if let Neutral::Head(ref head) = **neutral {
                return Some((head, spine));
            }
        }
        None
    }

    pub fn global_app(&self) -> Option<(&str, &Spine)> {
        self.head_app().and_then(|(head, spine)| match head {
            Head::Global(ref name) => Some((name.as_str(), spine)),
            Head::Extern(_, _) | Head::Var(_) => None,
        })
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.to_doc().group().render_fmt(pretty::FALLBACK_WIDTH, f)
    }
}

/// Reference counted values
#[derive(Debug, Clone, PartialEq, BoundTerm)]
pub struct RcValue {
    pub inner: Rc<Value>,
}

impl From<Value> for RcValue {
    fn from(src: Value) -> RcValue {
        RcValue {
            inner: Rc::new(src),
        }
    }
}

impl ops::Deref for RcValue {
    type Target = Value;

    fn deref(&self) -> &Value {
        &self.inner
    }
}

impl fmt::Display for RcValue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.inner, f)
    }
}

/// The head of an application
#[derive(Debug, Clone, PartialEq, BoundTerm)]
pub enum Head {
    /// Variables that have not yet been replaced with a definition
    Var(Var<String>),
    /// External definitions
    Extern(String, RcType),
    /// A global name
    Global(String),
    // TODO: Metavariables
}

/// The spine of a neutral term
///
/// These are arguments that are awaiting application
pub type Spine = Vec<RcValue>;

/// Neutral values
///
/// These might be able to be reduced further depending on the bindings in the
/// context
#[derive(Debug, Clone, PartialEq, BoundTerm)]
pub enum Neutral {
    /// Head of an application
    Head(Head),
    /// Field projection
    Proj(RcNeutral, Label),
    /// Case expressions
    Case(RcNeutral, Vec<Scope<RcPattern, RcValue>>),
}

impl Neutral {
    pub fn global(name: impl Into<String>) -> Neutral {
        Neutral::Head(Head::Global(name.into()))
    }
}

impl fmt::Display for Neutral {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.to_doc().group().render_fmt(pretty::FALLBACK_WIDTH, f)
    }
}

/// Reference counted neutral values
#[derive(Debug, Clone, PartialEq, BoundTerm)]
pub struct RcNeutral {
    pub inner: Rc<Neutral>,
}

impl From<Neutral> for RcNeutral {
    fn from(src: Neutral) -> RcNeutral {
        RcNeutral {
            inner: Rc::new(src),
        }
    }
}

impl ops::Deref for RcNeutral {
    type Target = Neutral;

    fn deref(&self) -> &Neutral {
        &self.inner
    }
}

/// Types are at the term level, so this is just an alias
pub type Type = Value;

/// Types are at the term level, so this is just an alias
pub type RcType = RcValue;

impl From<Var<String>> for Neutral {
    fn from(src: Var<String>) -> Neutral {
        Neutral::Head(Head::Var(src))
    }
}

impl From<Var<String>> for Value {
    fn from(src: Var<String>) -> Value {
        Value::from(Neutral::from(src))
    }
}

impl From<Neutral> for Value {
    fn from(src: Neutral) -> Value {
        Value::Neutral(RcNeutral::from(src), Spine::new())
    }
}

impl<'a> From<&'a Value> for Term {
    fn from(src: &'a Value) -> Term {
        // Bypassing `Scope::new` and `Scope::unbind` here should be fine
        // because we aren't altering the structure of the scopes during this
        // transformation. This should save on some traversals of the AST!
        match *src {
            Value::Universe(level) => Term::Universe(level),
            Value::Literal(ref lit) => Term::Literal(lit.clone()),
            Value::Pi(ref scope) => {
                let (ref name, Embed(ref ann)) = scope.unsafe_pattern;
                Term::Pi(Scope {
                    unsafe_pattern: (name.clone(), Embed(RcTerm::from(&**ann))),
                    unsafe_body: RcTerm::from(&*scope.unsafe_body),
                })
            },
            Value::Lam(ref scope) => {
                let (ref name, Embed(ref ann)) = scope.unsafe_pattern;
                Term::Lam(Scope {
                    unsafe_pattern: (name.clone(), Embed(RcTerm::from(&**ann))),
                    unsafe_body: RcTerm::from(&*scope.unsafe_body),
                })
            },
            Value::RecordType(ref scope) => {
                let unsafe_patterns = scope
                    .unsafe_pattern
                    .unsafe_patterns
                    .iter()
                    .map(|&(ref label, ref binder, Embed(ref ann))| {
                        (label.clone(), binder.clone(), Embed(RcTerm::from(&**ann)))
                    }).collect();

                Term::RecordType(Scope {
                    unsafe_pattern: Nest { unsafe_patterns },
                    unsafe_body: (),
                })
            },
            Value::Record(ref scope) => {
                let unsafe_patterns = scope
                    .unsafe_pattern
                    .unsafe_patterns
                    .iter()
                    .map(|&(ref label, ref binder, Embed(ref expr))| {
                        (label.clone(), binder.clone(), Embed(RcTerm::from(&**expr)))
                    }).collect();

                Term::Record(Scope {
                    unsafe_pattern: Nest { unsafe_patterns },
                    unsafe_body: (),
                })
            },
            Value::Array(ref elems) => {
                Term::Array(elems.iter().map(|elem| RcTerm::from(&**elem)).collect())
            },
            Value::Neutral(ref neutral, ref spine) => {
                spine.iter().fold(Term::from(&*neutral.inner), |acc, arg| {
                    Term::App(RcTerm::from(acc), RcTerm::from(&**arg))
                })
            },
        }
    }
}

impl<'a> From<&'a Value> for RcTerm {
    fn from(src: &'a Value) -> RcTerm {
        RcTerm::from(Term::from(src))
    }
}

impl<'a> From<&'a Neutral> for Term {
    fn from(src: &'a Neutral) -> Term {
        match *src {
            Neutral::Head(ref head) => Term::from(head),
            Neutral::Proj(ref expr, ref name) => Term::Proj(RcTerm::from(&**expr), name.clone()),
            Neutral::Case(ref head, ref clauses) => Term::Case(
                RcTerm::from(&**head),
                clauses
                    .iter()
                    .map(|clause| Scope {
                        unsafe_pattern: clause.unsafe_pattern.clone(),
                        unsafe_body: RcTerm::from(&*clause.unsafe_body),
                    }).collect(),
            ),
        }
    }
}

impl<'a> From<&'a Neutral> for RcTerm {
    fn from(src: &'a Neutral) -> RcTerm {
        RcTerm::from(Term::from(src))
    }
}

impl<'a> From<&'a Head> for Term {
    fn from(src: &'a Head) -> Term {
        match *src {
            Head::Var(ref var) => Term::Var(var.clone()),
            Head::Extern(ref name, ref ty) => Term::Extern(name.clone(), RcTerm::from(&**ty)),
            Head::Global(ref name) => Term::Global(name.clone()),
        }
    }
}
