//! Models top-level items, declarations, imports, classes, and parameters.

use oxc_ast::ast::{
    BindingPattern, Class, ClassElement, Declaration, FormalParameter, FormalParameters, Function,
    ImportDeclaration, ImportDeclarationSpecifier, ImportOrExportKind, MethodDefinition,
    MethodDefinitionKind, PropertyKey, Statement, VariableDeclaration, VariableDeclarationKind,
};
use oxc_span::GetSpan;

use crate::EcmaError;
use crate::extract::expressions::expression;
use crate::extract::kinds::{binding_kind, class_element_kind, declaration_kind, statement_kind};
use crate::extract::statements::statements;
use crate::extract::unsupported;
use crate::source::{
    EcmaBinding, EcmaClass, EcmaDeclaration, EcmaFunction, EcmaImport, EcmaImportedName, EcmaItem,
    EcmaMethod, EcmaParameter, EcmaVariable, EcmaVariableKind,
};

/// Models every top-level item of a module.
///
/// - `body`: Top-level statements in source order.
///
/// # Returns
/// The modeled items in the same order.
///
/// # Errors
/// Returns a structured error for the first unsupported construct.
pub(crate) fn items(body: &[Statement<'_>]) -> Result<Vec<EcmaItem>, EcmaError> {
    body.iter().map(item).collect()
}

/// Models one top-level item.
///
/// - `statement`: Top-level statement to model.
///
/// # Returns
/// The modeled item.
///
/// # Errors
/// Returns a structured error for an unsupported module form or a declaration
/// body that only exists inside a function or class.
pub(crate) fn item(statement: &Statement<'_>) -> Result<EcmaItem, EcmaError> {
    match statement {
        Statement::ImportDeclaration(declaration) => Ok(EcmaItem::Import(import(declaration)?)),
        Statement::ExportDeclaration(declaration) => {
            Ok(EcmaItem::Exported(exported(&declaration.declaration)?))
        }
        Statement::FunctionDeclaration(value) => Ok(EcmaItem::Declaration(
            EcmaDeclaration::Function(function(value)?),
        )),
        Statement::ClassDeclaration(value) => {
            Ok(EcmaItem::Declaration(EcmaDeclaration::Class(class(value)?)))
        }
        Statement::VariableDeclaration(declaration) => Ok(EcmaItem::Declaration(
            EcmaDeclaration::Variable(variable(declaration)?),
        )),
        Statement::ExpressionStatement(_) | Statement::IfStatement(_) => Ok(EcmaItem::Statement(
            crate::extract::statements::statement(statement)?,
        )),
        other => Err(unsupported(statement_kind(other), other.span())),
    }
}

/// Models the declaration of an `export` item.
///
/// - `declaration`: Declaration carried by `export`.
///
/// # Returns
/// The modeled declaration.
///
/// # Errors
/// Returns a structured error for a declaration `export` cannot carry here.
fn exported(declaration: &Declaration<'_>) -> Result<EcmaDeclaration, EcmaError> {
    match declaration {
        Declaration::FunctionDeclaration(value) => Ok(EcmaDeclaration::Function(function(value)?)),
        Declaration::ClassDeclaration(value) => Ok(EcmaDeclaration::Class(class(value)?)),
        Declaration::VariableDeclaration(value) => Ok(EcmaDeclaration::Variable(variable(value)?)),
        other => Err(unsupported(declaration_kind(other), other.span())),
    }
}

/// Models one `import` declaration.
///
/// - `declaration`: Import declaration to model.
///
/// # Returns
/// The module specifier and the bindings it introduces.
///
/// # Errors
/// Returns a structured error for namespace imports, type-only imports, import
/// attributes, or an import phase.
fn import(declaration: &ImportDeclaration<'_>) -> Result<EcmaImport, EcmaError> {
    if declaration.import_kind != ImportOrExportKind::Value {
        return Err(unsupported("type-only import", declaration.span));
    }
    if declaration.with_clause.is_some() || declaration.phase.is_some() {
        return Err(unsupported("import attributes", declaration.span));
    }
    let mut modeled = EcmaImport {
        source: declaration.source.value.as_str().to_owned(),
        default: None,
        named: Vec::new(),
    };
    for specifier in declaration.specifiers.iter().flatten() {
        match specifier {
            ImportDeclarationSpecifier::ImportDefaultSpecifier(value) => {
                if modeled.default.is_some() {
                    return Err(unsupported("repeated default import", value.span));
                }
                modeled.default = Some(value.local.name.as_str().to_owned());
            }
            ImportDeclarationSpecifier::ImportSpecifier(value) => {
                let imported = match &value.imported {
                    oxc_ast::ast::ModuleExportName::IdentifierName(name) => {
                        name.name.as_str().to_owned()
                    }
                    oxc_ast::ast::ModuleExportName::IdentifierReference(name) => {
                        name.name.as_str().to_owned()
                    }
                    oxc_ast::ast::ModuleExportName::StringLiteral(literal) => {
                        literal.value.as_str().to_owned()
                    }
                };
                let alias = (value.local.name.as_str() != imported)
                    .then(|| value.local.name.as_str().to_owned());
                modeled.named.push(EcmaImportedName {
                    imported,
                    local: alias,
                });
            }
            ImportDeclarationSpecifier::ImportNamespaceSpecifier(value) => {
                return Err(unsupported("namespace import", value.span));
            }
        }
    }
    Ok(modeled)
}

/// Models one variable declaration.
///
/// - `declaration`: Variable declaration to model.
///
/// # Returns
/// The declaration keyword family and its bindings.
///
/// # Errors
/// Returns a structured error for `using` keywords, destructuring bindings,
/// type annotations, definite assignment markers, or unsupported initializers.
pub(crate) fn variable(declaration: &VariableDeclaration<'_>) -> Result<EcmaVariable, EcmaError> {
    let kind = match declaration.kind {
        VariableDeclarationKind::Var => EcmaVariableKind::Var,
        VariableDeclarationKind::Let => EcmaVariableKind::Let,
        VariableDeclarationKind::Const => EcmaVariableKind::Const,
        VariableDeclarationKind::Using => {
            return Err(unsupported("using declaration", declaration.span));
        }
        VariableDeclarationKind::AwaitUsing => {
            return Err(unsupported("await using declaration", declaration.span));
        }
    };
    let mut bindings = Vec::new();
    for declarator in &declaration.declarations {
        if declarator.type_annotation.is_some() || declarator.definite {
            return Err(unsupported("typed variable declaration", declarator.span));
        }
        bindings.push(EcmaBinding {
            name: binding_name(&declarator.id)?,
            initializer: declarator.init.as_ref().map(expression).transpose()?,
        });
    }
    Ok(EcmaVariable { kind, bindings })
}

/// Models one function declaration.
///
/// - `function`: Function declaration to model.
///
/// # Returns
/// The name, parameters, and body statements.
///
/// # Errors
/// Returns a structured error for anonymous, generator, async, typed, or
/// overloads-without-a-body functions, and for function directives.
fn function(function: &Function<'_>) -> Result<EcmaFunction, EcmaError> {
    if function.generator || function.r#async {
        return Err(unsupported("generator or async function", function.span));
    }
    if function.type_parameters.is_some() || function.return_type.is_some() {
        return Err(unsupported("typed function signature", function.span));
    }
    let Some(id) = &function.id else {
        return Err(unsupported("anonymous function", function.span));
    };
    let Some(body) = &function.body else {
        return Err(unsupported(
            "function declaration without a body",
            function.span,
        ));
    };
    if !body.directives.is_empty() {
        return Err(unsupported("directive", body.span));
    }
    Ok(EcmaFunction {
        name: id.name.as_str().to_owned(),
        parameters: parameters(&function.params)?,
        body: statements(&body.statements)?,
    })
}

/// Models one class declaration.
///
/// - `class`: Class declaration to model.
///
/// # Returns
/// The class name and its method members.
///
/// # Errors
/// Returns a structured error for anonymous classes, heritage clauses,
/// decorators, type parameters, `implements` clauses, and members that are not
/// plain instance or static-free methods.
fn class(class: &Class<'_>) -> Result<EcmaClass, EcmaError> {
    let Some(id) = &class.id else {
        return Err(unsupported("anonymous class", class.span));
    };
    if class.heritage.is_some()
        || !class.decorators.is_empty()
        || class.type_parameters.is_some()
        || !class.implements.is_empty()
    {
        return Err(unsupported("class heritage or decorated class", class.span));
    }
    let mut members = Vec::new();
    for member in &class.body.body {
        match member {
            ClassElement::MethodDefinition(value) => members.push(method(value)?),
            other => return Err(unsupported(class_element_kind(other), other.span())),
        }
    }
    Ok(EcmaClass {
        name: id.name.as_str().to_owned(),
        members,
    })
}

/// Models one class method member.
///
/// - `member`: Method definition to model.
///
/// # Returns
/// The method name, parameters, and body statements.
///
/// # Errors
/// Returns a structured error for getters, setters, static, computed, private,
/// optional, or abstract members and for typed method signatures.
fn method(member: &MethodDefinition<'_>) -> Result<EcmaMethod, EcmaError> {
    if !matches!(
        member.kind,
        MethodDefinitionKind::Method | MethodDefinitionKind::Constructor
    ) {
        return Err(unsupported("class getter or setter", member.span));
    }
    if member.r#static || member.computed || member.optional || member.r#override {
        return Err(unsupported("static or computed class member", member.span));
    }
    if member.accessibility.is_some() || !member.decorators.is_empty() {
        return Err(unsupported("class member with type modifiers", member.span));
    }
    if member.value.r#async || member.value.generator {
        return Err(unsupported("async or generator class member", member.span));
    }
    if member.value.type_parameters.is_some() || member.value.return_type.is_some() {
        return Err(unsupported("typed method signature", member.span));
    }
    let name = match &member.key {
        PropertyKey::StaticIdentifier(identifier) => identifier.name.as_str().to_owned(),
        other => {
            return Err(unsupported(
                "computed or private class member key",
                other.span(),
            ));
        }
    };
    let Some(body) = &member.value.body else {
        return Err(unsupported("method without a body", member.span));
    };
    if !body.directives.is_empty() {
        return Err(unsupported("directive", body.span));
    }
    Ok(EcmaMethod {
        name,
        parameters: parameters(&member.value.params)?,
        body: statements(&body.statements)?,
    })
}

/// Models a parameter list of plain identifier bindings.
///
/// - `parameters`: Parameter list to model.
///
/// # Returns
/// The modeled parameters in declaration order.
///
/// # Errors
/// Returns a structured error for rest, default, destructuring, or typed
/// parameters and for parameter decorators.
pub(crate) fn parameters(
    parameters: &FormalParameters<'_>,
) -> Result<Vec<EcmaParameter>, EcmaError> {
    if parameters.rest.is_some() {
        return Err(unsupported("rest parameter", parameters.span));
    }
    parameters
        .items
        .iter()
        .map(|parameter| match parameter {
            FormalParameter {
                initializer: Some(_),
                ..
            } => Err(unsupported("default parameter", parameter.span)),
            FormalParameter {
                type_annotation: Some(_),
                ..
            } => Err(unsupported("typed parameter", parameter.span)),
            FormalParameter { optional: true, .. } => {
                Err(unsupported("optional parameter", parameter.span))
            }
            FormalParameter {
                accessibility: Some(_),
                ..
            } => Err(unsupported("parameter property", parameter.span)),
            value => {
                if !value.decorators.is_empty() || value.r#override || value.readonly {
                    return Err(unsupported("parameter with type modifiers", value.span));
                }
                Ok(EcmaParameter {
                    name: binding_name(&value.pattern)?,
                })
            }
        })
        .collect()
}

/// Reads the identifier name bound by a simple binding pattern.
///
/// - `pattern`: Binding pattern to read.
///
/// # Returns
/// The bound identifier name.
///
/// # Errors
/// Returns a structured error for any destructuring or type-annotated pattern.
pub(crate) fn binding_name(pattern: &BindingPattern<'_>) -> Result<String, EcmaError> {
    match pattern {
        BindingPattern::BindingIdentifier(identifier) => Ok(identifier.name.as_str().to_owned()),
        other => Err(unsupported(binding_kind(other), other.span())),
    }
}
