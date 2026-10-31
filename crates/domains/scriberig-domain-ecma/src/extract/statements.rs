//! Models statements that appear inside function, method, and block bodies.

use oxc_ast::ast::Statement;
use oxc_span::GetSpan;

use crate::EcmaError;
use crate::extract::declarations::variable;
use crate::extract::expressions::expression;
use crate::extract::kinds::statement_kind;
use crate::extract::unsupported;
use crate::source::{EcmaAlternate, EcmaIf, EcmaReturn, EcmaStatement};

/// Models a statement list in source order.
///
/// - `body`: Statements to model.
///
/// # Returns
/// The modeled statements.
///
/// # Errors
/// Returns a structured error for the first unsupported construct.
pub(crate) fn statements(body: &[Statement<'_>]) -> Result<Vec<EcmaStatement>, EcmaError> {
    body.iter().map(statement).collect()
}

/// Models one statement of a body.
///
/// - `statement`: Statement to model.
///
/// # Returns
/// The modeled statement.
///
/// # Errors
/// Returns a structured error for loops, `try`, `switch`, `throw`, labels,
/// empty statements, nested declarations, and any unsupported expression.
pub(crate) fn statement(statement: &Statement<'_>) -> Result<EcmaStatement, EcmaError> {
    match statement {
        Statement::VariableDeclaration(declaration) => {
            Ok(EcmaStatement::Variable(variable(declaration)?))
        }
        Statement::ExpressionStatement(value) => {
            Ok(EcmaStatement::Expression(expression(&value.expression)?))
        }
        Statement::ReturnStatement(value) => Ok(EcmaStatement::Return(EcmaReturn {
            value: value.argument.as_ref().map(expression).transpose()?,
        })),
        Statement::IfStatement(value) => Ok(EcmaStatement::If(if_statement(value)?)),
        Statement::BlockStatement(value) => Ok(EcmaStatement::Block(statements(&value.body)?)),
        other => Err(unsupported(statement_kind(other), other.span())),
    }
}

/// Models one `if` statement.
///
/// - `statement`: `if` statement to model.
///
/// # Returns
/// The condition, consequent block, and optional `else` branch.
///
/// # Errors
/// Returns a structured error when a branch is written without braces or when
/// the condition is outside the supported expression subset.
fn if_statement(statement: &oxc_ast::ast::IfStatement<'_>) -> Result<EcmaIf, EcmaError> {
    let consequent = block_of(&statement.consequent)?;
    let alternate = statement
        .alternate
        .as_ref()
        .map(|branch| match branch {
            Statement::IfStatement(nested) => {
                Ok(EcmaAlternate::ElseIf(Box::new(if_statement(nested)?)))
            }
            other => Ok(EcmaAlternate::Block(block_of(other)?)),
        })
        .transpose()?;
    Ok(EcmaIf {
        test: expression(&statement.test)?,
        consequent,
        alternate,
    })
}

/// Models the braced statement list of an `if` branch.
///
/// - `statement`: Statement used as an `if` branch.
///
/// # Returns
/// The statements of the branch block.
///
/// # Errors
/// Returns a structured error when the branch is a single unbraced statement.
fn block_of(statement: &Statement<'_>) -> Result<Vec<EcmaStatement>, EcmaError> {
    match statement {
        Statement::BlockStatement(block) => statements(&block.body),
        other => Err(unsupported(
            &format!("if branch without a block: {}", statement_kind(other)),
            other.span(),
        )),
    }
}
