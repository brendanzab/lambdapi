//! The syntax of the language, unchecked and with implicit parts that need to
//! be elaborated in a type-directed way during type checking and inference

use codespan::ByteSpan;
use moniker::{Binder, Embed, Nest, Scope, Var};
use std::fmt;
use std::ops;
use std::rc::Rc;

use pretty::{self, ToDoc};
use {FloatFormat, IntFormat, Label, Level, LevelShift};

/// Literals
#[derive(Debug, Clone, PartialEq, PartialOrd, BoundTerm, BoundPattern)]
pub enum Literal {
    String(ByteSpan, String),
    Char(ByteSpan, char),
    Int(ByteSpan, u64, IntFormat),
    Float(ByteSpan, f64, FloatFormat),
}

impl Literal {
    /// Return the span of source code that the literal originated from
    pub fn span(&self) -> ByteSpan {
        match *self {
            Literal::String(span, ..)
            | Literal::Char(span, ..)
            | Literal::Int(span, ..)
            | Literal::Float(span, ..) => span,
        }
    }
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
    Binder(ByteSpan, Binder<String>),
    /// Patterns to be compared structurally with a variable in scope
    Var(ByteSpan, Embed<Var<String>>, LevelShift),
    /// Literal patterns
    Literal(Literal),
}

impl Pattern {
    /// Return the span of source code that this pattern originated from
    pub fn span(&self) -> ByteSpan {
        match *self {
            Pattern::Ann(ref pattern, Embed(ref ty)) => pattern.span().to(ty.span()),
            Pattern::Var(span, _, _) | Pattern::Binder(span, _) => span,
            Pattern::Literal(ref literal) => literal.span(),
        }
    }
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

/// Terms, unchecked and with implicit syntax that needs to be elaborated
///
/// For now the only implicit syntax we have is holes and lambdas that lack a
/// type annotation.
#[derive(Debug, Clone, PartialEq, BoundTerm)]
pub enum Term {
    /// A term annotated with a type
    Ann(RcTerm, RcTerm),
    /// Universes
    Universe(ByteSpan, Level),
    /// Literals
    Literal(Literal),
    /// A hole
    Hole(ByteSpan),
    /// A variable
    Var(ByteSpan, Var<String>, LevelShift),
    /// An imported definition
    Import(ByteSpan, ByteSpan, String),
    /// Dependent function types
    Pi(ByteSpan, Scope<(Binder<String>, Embed<RcTerm>), RcTerm>),
    /// Lambda abstractions
    Lam(ByteSpan, Scope<(Binder<String>, Embed<RcTerm>), RcTerm>),
    /// Term application
    App(RcTerm, RcTerm),
    /// Dependent record types
    RecordType(
        ByteSpan,
        Scope<Nest<(Label, Binder<String>, Embed<RcTerm>)>, ()>,
    ),
    /// Dependent record introduction
    Record(
        ByteSpan,
        Scope<Nest<(Label, Binder<String>, Embed<RcTerm>)>, ()>,
    ),
    /// Record field projection
    Proj(ByteSpan, RcTerm, ByteSpan, Label, LevelShift),
    /// Variant types
    VariantType(ByteSpan, Vec<(Label, RcTerm)>),
    /// Variant introduction
    Variant(ByteSpan, Label, RcTerm),
    /// Case expressions
    Case(ByteSpan, RcTerm, Vec<Scope<RcPattern, RcTerm>>),
    /// Array literals
    Array(ByteSpan, Vec<RcTerm>),
    /// Let bindings
    Let(
        ByteSpan,
        Scope<Nest<(Binder<String>, Embed<(RcTerm, RcTerm)>)>, RcTerm>,
    ),
}

impl Term {
    pub fn span(&self) -> ByteSpan {
        match *self {
            Term::Universe(span, ..)
            | Term::Hole(span)
            | Term::Var(span, ..)
            | Term::Import(span, ..)
            | Term::Pi(span, ..)
            | Term::Lam(span, ..)
            | Term::RecordType(span, ..)
            | Term::Record(span, ..)
            | Term::Proj(span, ..)
            | Term::VariantType(span, ..)
            | Term::Variant(span, ..)
            | Term::Case(span, ..)
            | Term::Array(span, ..)
            | Term::Let(span, ..) => span,
            Term::Literal(ref literal) => literal.span(),
            Term::Ann(ref expr, ref ty) => expr.span().to(ty.span()),
            Term::App(ref head, ref arg) => head.span().to(arg.span()),
        }
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
