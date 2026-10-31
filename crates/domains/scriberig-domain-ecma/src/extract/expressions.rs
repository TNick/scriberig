//! Models the supported expression subset: literals, identifiers, member
//! accesses, calls, `=` assignment, and arrow functions.

use oxc_ast::ast::{
    Argument, ArrowFunctionExpression, AssignmentOperator, AssignmentTarget, Expression,
    NumericLiteral, SimpleAssignmentTarget, StaticMemberExpression,
};
use oxc_span::GetSpan;

use crate::EcmaError;
use crate::extract::declarations::parameters;
use crate::extract::kinds::expression_kind;
use crate::extract::statements::statements;
use crate::extract::unsupported;
use crate::source::{EcmaArrow, EcmaArrowBody, EcmaExpression, EcmaLiteral};

/// Models one expression.
///
/// - `node`: Expression to model.
///
/// # Returns
/// The modeled expression.
///
/// # Errors
/// Returns a structured error for operators, object, array, template, regexp,
/// and bigint literals, `this`, `new`, spread, optional chaining, and type
/// assertions.
pub(crate) fn expression(node: &Expression<'_>) -> Result<EcmaExpression, EcmaError> {
    match node {
        Expression::Identifier(identifier) => Ok(EcmaExpression::Identifier(
            identifier.name.as_str().to_owned(),
        )),
        Expression::NumericLiteral(literal) => Ok(EcmaExpression::Literal(EcmaLiteral::Number(
            number_text(literal)?,
        ))),
        Expression::StringLiteral(literal) => Ok(EcmaExpression::Literal(EcmaLiteral::String(
            literal.value.as_str().to_owned(),
        ))),
        Expression::BooleanLiteral(literal) => {
            Ok(EcmaExpression::Literal(EcmaLiteral::Boolean(literal.value)))
        }
        Expression::NullLiteral(_) => Ok(EcmaExpression::Literal(EcmaLiteral::Null)),
        Expression::StaticMemberExpression(member) => Ok(EcmaExpression::Member {
            object: Box::new(expression(&member.object)?),
            property: member.property.name.as_str().to_owned(),
        }),
        Expression::CallExpression(call) => {
            if call.optional {
                return Err(unsupported("optional call", call.span));
            }
            Ok(EcmaExpression::Call {
                callee: Box::new(expression(&call.callee)?),
                arguments: call
                    .arguments
                    .iter()
                    .map(argument)
                    .collect::<Result<Vec<_>, _>>()?,
            })
        }
        Expression::AssignmentExpression(assignment) => {
            if assignment.operator != AssignmentOperator::Assign {
                return Err(unsupported(
                    &format!("compound assignment {}", assignment.operator.as_str()),
                    assignment.span,
                ));
            }
            Ok(EcmaExpression::Assignment {
                target: Box::new(assignment_target(&assignment.left)?),
                value: Box::new(expression(&assignment.right)?),
            })
        }
        Expression::ArrowFunctionExpression(value) => Ok(EcmaExpression::Arrow(arrow(value)?)),
        other => Err(unsupported(expression_kind(other), other.span())),
    }
}

/// Models one call argument.
///
/// - `argument`: Argument to model.
///
/// # Returns
/// The modeled argument expression.
///
/// # Errors
/// Returns a structured error for spread arguments.
fn argument(argument: &Argument<'_>) -> Result<EcmaExpression, EcmaError> {
    match argument {
        Argument::SpreadElement(spread) => Err(unsupported("spread argument", spread.span)),
        other => expression(other.to_expression()),
    }
}

/// Models one assignment target.
///
/// - `target`: Assignment target to model.
///
/// # Returns
/// The modeled identifier or member access.
///
/// # Errors
/// Returns a structured error for destructuring targets, computed members,
/// private fields, and type-asserted targets.
fn assignment_target(target: &AssignmentTarget<'_>) -> Result<EcmaExpression, EcmaError> {
    let Some(simple) = target.as_simple_assignment_target() else {
        return Err(unsupported(
            "destructuring assignment target",
            target.span(),
        ));
    };
    match simple {
        SimpleAssignmentTarget::AssignmentTargetIdentifier(identifier) => Ok(
            EcmaExpression::Identifier(identifier.name.as_str().to_owned()),
        ),
        SimpleAssignmentTarget::StaticMemberExpression(member) => member_access(member),
        other => Err(unsupported(
            "destructuring or computed assignment target",
            other.span(),
        )),
    }
}

/// Models one static member access.
///
/// - `member`: Member expression to model.
///
/// # Returns
/// The receiver and property name.
///
/// # Errors
/// Returns a structured error for optional chaining.
fn member_access(member: &StaticMemberExpression<'_>) -> Result<EcmaExpression, EcmaError> {
    if member.optional {
        return Err(unsupported("optional member access", member.span));
    }
    Ok(EcmaExpression::Member {
        object: Box::new(expression(&member.object)?),
        property: member.property.name.as_str().to_owned(),
    })
}

/// Models one arrow function.
///
/// - `arrow`: Arrow function to model.
///
/// # Returns
/// The parameters and the expression or block body.
///
/// # Errors
/// Returns a structured error for async arrows, arrow type parameters, arrow
/// return types, and unsupported parameters or body statements.
fn arrow(arrow: &ArrowFunctionExpression<'_>) -> Result<EcmaArrow, EcmaError> {
    if arrow.r#async {
        return Err(unsupported("async arrow function", arrow.span));
    }
    if arrow.type_parameters.is_some() || arrow.return_type.is_some() {
        return Err(unsupported("typed arrow function", arrow.span));
    }
    let body = match arrow.body.as_function_body() {
        Some(body) => {
            if !body.directives.is_empty() {
                return Err(unsupported("directive", body.span));
            }
            EcmaArrowBody::Block(statements(&body.statements)?)
        }
        None => EcmaArrowBody::Expression(Box::new(expression(
            arrow
                .body
                .as_expression()
                .ok_or_else(|| unsupported("arrow function without a body", arrow.span))?,
        )?)),
    };
    Ok(EcmaArrow {
        parameters: parameters(&arrow.params)?,
        body,
    })
}

/// Reads the exact source digits of a numeric literal.
///
/// - `literal`: Numeric literal to read.
///
/// # Returns
/// The literal text as written in the source.
///
/// # Errors
/// Returns a structured error when the literal carries no source text, because
/// re-formatted floats would change the modeled value.
fn number_text(literal: &NumericLiteral<'_>) -> Result<String, EcmaError> {
    literal
        .raw
        .as_ref()
        .map(|raw| raw.as_str().to_owned())
        .ok_or_else(|| unsupported("numeric literal without source text", literal.span))
}
