//! The semantics of the language
//!
//! Here we define the rules of normalization, type checking, and type inference.
//!
//! For more information, check out the theory appendix of the Pikelet book.

use codespan::ByteSpan;
use nameless::{self, BoundTerm, Embed, Name, Var};
use std::rc::Rc;

use syntax::core::{Constant, Context, Definition, Level, Module, RawConstant, RawModule, RawTerm,
                   Term, Type, Value};
use syntax::translation::Resugar;

mod conversion;
mod errors;
mod normalize;
#[cfg(test)]
mod tests;

pub use self::conversion::{compare, compare_whnf};
pub use self::errors::{InternalError, TypeError};
pub use self::normalize::whnf;

/// Typecheck and elaborate a module
pub fn check_module(raw_module: &RawModule) -> Result<Module, TypeError> {
    let mut context = Context::default();
    let mut definitions = Vec::with_capacity(raw_module.definitions.len());

    for raw_definition in &raw_module.definitions {
        let name = raw_definition.name.clone();
        let (term, ann) = match *raw_definition.ann {
            // We don't have a type annotation available to us! Instead we will
            // attempt to infer it based on the body of the definition
            RawTerm::Hole(_) => infer(&context, &raw_definition.term)?,
            // We have a type annotation! Elaborate it, then nomalize it, then
            // check that it matches the body of the definition
            _ => {
                let (ann, _) = infer(&context, &raw_definition.ann)?;
                let ann = whnf(&context, &ann)?;
                let term = check(&context, &raw_definition.term, &ann)?;
                (term, ann)
            },
        };

        // Add the definition to the context
        context = context.claim(Name::user(name.clone()), ann.clone());
        context = context.define(Name::user(name.clone()), term.clone());

        definitions.push(Definition { name, term, ann })
    }

    Ok(Module {
        name: raw_module.name.clone(),
        definitions,
    })
}

/// Type checking of terms
pub fn check(
    context: &Context,
    raw_term: &Rc<RawTerm>,
    expected_ty: &Rc<Type>,
) -> Result<Rc<Term>, TypeError> {
    match (&**raw_term, &**expected_ty) {
        (&RawTerm::Constant(span, ref raw_c), &Value::Constant(ref c_ty)) => {
            use syntax::core::RawConstant as RawC;

            let c = match (raw_c, c_ty) {
                (&RawC::String(ref val), &Constant::StringType) => Constant::String(val.clone()),
                (&RawC::Char(val), &Constant::CharType) => Constant::Char(val),

                // FIXME: overflow?
                (&RawC::Int(val), &Constant::U8Type) => Constant::U8(val as u8),
                (&RawC::Int(val), &Constant::U16Type) => Constant::U16(val as u16),
                (&RawC::Int(val), &Constant::U32Type) => Constant::U32(val as u32),
                (&RawC::Int(val), &Constant::U64Type) => Constant::U64(val),
                (&RawC::Int(val), &Constant::I8Type) => Constant::I8(val as i8),
                (&RawC::Int(val), &Constant::I16Type) => Constant::I16(val as i16),
                (&RawC::Int(val), &Constant::I32Type) => Constant::I32(val as i32),
                (&RawC::Int(val), &Constant::I64Type) => Constant::I64(val as i64),
                (&RawC::Int(val), &Constant::F32Type) => Constant::F32(val as f32),
                (&RawC::Int(val), &Constant::F64Type) => Constant::F64(val as f64),
                (&RawC::Float(val), &Constant::F32Type) => Constant::F32(val as f32),
                (&RawC::Float(val), &Constant::F64Type) => Constant::F64(val),

                (_, _) => {
                    return Err(TypeError::LiteralMismatch {
                        literal_span: span.0,
                        found: raw_c.clone(),
                        expected: Box::new(c_ty.resugar()),
                    });
                },
            };

            return Ok(Rc::new(Term::Constant(span, c)));
        },

        // C-LAM
        (&RawTerm::Lam(span, ref lam_scope), &Value::Pi(ref pi_scope)) => {
            let ((lam_name, Embed(lam_ann)), lam_body, (pi_name, Embed(pi_ann)), pi_body) =
                nameless::unbind2(lam_scope.clone(), pi_scope.clone());

            // Elaborate the hole, if it exists
            if let RawTerm::Hole(_) = *lam_ann {
                let lam_ann = whnf(context, &pi_ann)?;
                let pi_body = whnf(context, &pi_body)?;
                let lam_body = check(&context.claim(pi_name, lam_ann), &lam_body, &pi_body)?;

                return Ok(Rc::new(Term::Lam(
                    span,
                    nameless::bind((lam_name, Embed(pi_ann)), lam_body),
                )));
            }

            // TODO: We might want to optimise for this case, rather than
            // falling through to `infer` and reunbinding at I-LAM
        },
        (&RawTerm::Lam(_, _), _) => {
            return Err(TypeError::UnexpectedFunction {
                span: raw_term.span(),
                expected: Box::new(expected_ty.resugar()),
            });
        },

        // C-IF
        (&RawTerm::If(span, ref raw_cond, ref raw_if_true, ref raw_if_false), _) => {
            let bool_ty = Rc::new(Value::Constant(Constant::BoolType));
            let cond = check(context, raw_cond, &bool_ty)?;
            let if_true = check(context, raw_if_true, expected_ty)?;
            let if_false = check(context, raw_if_false, expected_ty)?;

            return Ok(Rc::new(Term::If(span, cond, if_true, if_false)));
        },

        // C-RECORD
        (
            &RawTerm::Record(span, ref label, ref raw_expr, ref raw_rest),
            &Value::RecordType(ref ty_label, ref ann, ref ty_rest),
        ) => {
            if label == ty_label {
                let ann = whnf(context, ann)?;
                let expr = check(context, &raw_expr, &ann)?;
                let body = check(context, &raw_rest, &ty_rest)?;

                return Ok(Rc::new(Term::Record(span, label.clone(), expr, body)));
            } else {
                unimplemented!()
            }
        },

        (&RawTerm::Hole(span), _) => {
            return Err(TypeError::UnableToElaborateHole {
                span: span.0,
                expected: Some(Box::new(expected_ty.resugar())),
            });
        },

        _ => {},
    }

    // C-CONV
    let (term, inferred_ty) = infer(context, raw_term)?;
    match Type::term_eq(&inferred_ty, expected_ty) {
        true => Ok(term),
        false => Err(TypeError::Mismatch {
            span: term.span(),
            found: Box::new(inferred_ty.resugar()),
            expected: Box::new(expected_ty.resugar()),
        }),
    }
}

/// Type inference of terms
pub fn infer(context: &Context, raw_term: &Rc<RawTerm>) -> Result<(Rc<Term>, Rc<Type>), TypeError> {
    use std::cmp;

    /// Ensures that the given term is a universe, returning the level of that
    /// universe and its elaborated form.
    fn infer_universe(
        context: &Context,
        raw_term: &Rc<RawTerm>,
    ) -> Result<(Rc<Term>, Level), TypeError> {
        let (term, ty) = infer(context, raw_term)?;
        match *ty {
            Value::Universe(level) => Ok((term, level)),
            _ => Err(TypeError::ExpectedUniverse {
                span: raw_term.span(),
                found: Box::new(ty.resugar()),
            }),
        }
    }

    match **raw_term {
        //  I-ANN
        RawTerm::Ann(span, ref raw_expr, ref raw_ty) => {
            let (ty, _) = infer_universe(context, raw_ty)?;
            let value_ty = whnf(context, &ty)?;
            let expr = check(context, raw_expr, &value_ty)?;

            Ok((Rc::new(Term::Ann(span, expr, ty)), value_ty))
        },

        // I-TYPE
        RawTerm::Universe(span, level) => Ok((
            Rc::new(Term::Universe(span, level)),
            Rc::new(Value::Universe(level.succ())),
        )),

        RawTerm::Hole(span) => Err(TypeError::UnableToElaborateHole {
            span: span.0,
            expected: None,
        }),

        RawTerm::Constant(span, ref raw_c) => match *raw_c {
            RawConstant::String(ref value) => Ok((
                Rc::new(Term::Constant(span, Constant::String(value.clone()))),
                Rc::new(Value::Constant(Constant::StringType)),
            )),
            RawConstant::Char(value) => Ok((
                Rc::new(Term::Constant(span, Constant::Char(value))),
                Rc::new(Value::Constant(Constant::CharType)),
            )),
            RawConstant::Int(_) => Err(TypeError::AmbiguousIntLiteral { span: span.0 }),
            RawConstant::Float(_) => Err(TypeError::AmbiguousFloatLiteral { span: span.0 }),
        },

        // I-VAR
        RawTerm::Var(span, ref var) => match *var {
            Var::Free(ref name) => match context.lookup_claim(name) {
                Some(ty) => Ok((Rc::new(Term::Var(span, var.clone())), ty.clone())),
                None => Err(TypeError::UndefinedName {
                    var_span: span.0,
                    name: name.clone(),
                }),
            },

            // We should always be substituting bound variables with fresh
            // variables when entering scopes using `unbind`, so if we've
            // encountered one here this is definitely a bug!
            Var::Bound(ref name, index) => Err(InternalError::UnsubstitutedDebruijnIndex {
                span: raw_term.span(),
                name: name.clone(),
                index: index,
            }.into()),
        },

        // I-PI
        RawTerm::Pi(span, ref raw_scope) => {
            let ((name, Embed(raw_ann)), raw_body) = nameless::unbind(raw_scope.clone());

            let (ann, ann_level) = infer_universe(context, &raw_ann)?;
            let (body, body_level) = {
                let ann = whnf(context, &ann)?;
                infer_universe(&context.claim(name.clone(), ann), &raw_body)?
            };

            Ok((
                Rc::new(Term::Pi(span, nameless::bind((name, Embed(ann)), body))),
                Rc::new(Value::Universe(cmp::max(ann_level, body_level))),
            ))
        },

        // I-LAM
        RawTerm::Lam(span, ref raw_scope) => {
            let ((name, Embed(raw_ann)), raw_body) = nameless::unbind(raw_scope.clone());

            // Check for holes before entering to ensure we get a nice error
            if let RawTerm::Hole(_) = *raw_ann {
                return Err(TypeError::FunctionParamNeedsAnnotation {
                    param_span: ByteSpan::default(), // TODO: param.span(),
                    var_span: None,
                    name: name.clone(),
                });
            }

            let (ann, _) = infer_universe(context, &raw_ann)?;
            let (body, body_ty) = infer(
                &context.claim(name.clone(), whnf(context, &ann)?),
                &raw_body,
            )?;

            Ok((
                Rc::new(Term::Lam(
                    span,
                    nameless::bind((name.clone(), Embed(ann.clone())), body),
                )),
                Rc::new(Value::Pi(nameless::bind(
                    (name, Embed(ann)),
                    Rc::new(Term::from(&*body_ty)),
                ))),
            ))
        },

        // I-APP
        RawTerm::App(ref raw_expr, ref raw_arg) => {
            let (expr, expr_ty) = infer(context, raw_expr)?;

            match *expr_ty {
                Value::Pi(ref scope) => {
                    let ((name, Embed(ann)), body) = nameless::unbind(scope.clone());

                    let ann = whnf(context, &ann)?;
                    let arg = check(context, raw_arg, &ann)?;
                    let body = whnf(
                        context,
                        &Rc::new(Term::Subst(nameless::bind(
                            (name, Embed(arg.clone())),
                            body,
                        ))),
                    )?;

                    Ok((Rc::new(Term::App(expr, arg)), body))
                },
                _ => Err(TypeError::ArgAppliedToNonFunction {
                    fn_span: raw_expr.span(),
                    arg_span: raw_arg.span(),
                    found: Box::new(expr_ty.resugar()),
                }),
            }
        },

        // I-IF
        RawTerm::If(span, ref raw_cond, ref raw_if_true, ref raw_if_false) => {
            let bool_ty = Rc::new(Value::Constant(Constant::BoolType));
            let cond = check(context, raw_cond, &bool_ty)?;
            let (if_true, ty) = infer(context, raw_if_true)?;
            let if_false = check(context, raw_if_false, &ty)?;

            Ok((Rc::new(Term::If(span, cond, if_true, if_false)), ty))
        },

        // I-RECORD-TYPE
        RawTerm::RecordType(span, ref label, ref raw_ann, ref raw_rest) => {
            // Check that rest of record type is well-formed?
            // Might be able to skip that for now, because there's no way to
            // express ill-formed records in the concrete syntax...

            let (ann, ann_level) = infer_universe(context, &raw_ann)?;
            let (rest, rest_level) = infer_universe(context, &raw_rest)?;

            Ok((
                Rc::new(Term::RecordType(span, label.clone(), ann, rest)),
                Rc::new(Value::Universe(cmp::max(ann_level, rest_level))),
            ))
        },

        // I-RECORD
        RawTerm::Record(span, ref label, ref raw_expr, ref raw_rest) => {
            // Check that rest of record is well-formed?
            // Might be able to skip that for now, because there's no way to
            // express ill-formed records in the concrete syntax...

            let (expr, ann) = infer(context, &raw_expr)?;
            let (rest, ty_rest) = infer(context, &raw_rest)?;

            Ok((
                Rc::new(Term::Record(span, label.clone(), expr, rest)),
                Rc::new(Value::RecordType(
                    label.clone(),
                    Rc::new(Term::from(&*ann)),
                    ty_rest,
                )),
            ))
        },

        // I-EMPTY-RECORD-TYPE
        RawTerm::EmptyRecordType(span) => Ok((
            Rc::new(Term::EmptyRecordType(span)),
            Rc::new(Value::Universe(Level(0))),
        )),

        // I-EMPTY-RECORD
        RawTerm::EmptyRecord(span) => Ok((
            Rc::new(Term::EmptyRecord(span)),
            Rc::new(Value::EmptyRecordType),
        )),

        // I-PROJ
        RawTerm::Proj(span, ref expr, label_span, ref label) => {
            let (expr, ty) = infer(context, expr)?;

            match ty.lookup_record_ty(label) {
                Some(ann) => {
                    let ann = whnf(context, &ann)?;
                    let expr = Rc::new(Term::Proj(span, expr, label_span, label.clone()));

                    Ok((expr, ann))
                },
                None => Err(TypeError::NoFieldInType {
                    label_span: label_span.0,
                    expected_label: label.clone(),
                    found: Box::new(ty.resugar()),
                }),
            }
        },
    }
}
