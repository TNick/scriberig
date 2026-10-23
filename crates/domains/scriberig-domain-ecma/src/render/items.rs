//! Renders top-level items: imports, exported and plain declarations.

use crate::EcmaError;
use crate::render::Writer;
use crate::render::expressions::expression_text;
use crate::render::statements::write_body;
use crate::source::{
    EcmaBinding, EcmaClass, EcmaDeclaration, EcmaFunction, EcmaImport, EcmaImportedName, EcmaItem,
    EcmaMethod, EcmaParameter, EcmaVariable, EcmaVariableKind,
};
use crate::style::EcmaStyle;

/// Writes every top-level item in source order.
///
/// - `writer`: Accumulating sink.
/// - `depth`: Nesting depth of the items.
/// - `items`: Items to render.
///
/// # Returns
/// Unit once every item is written.
///
/// # Errors
/// Returns an error when an item cannot be rendered.
pub(crate) fn write_items(
    writer: &mut Writer,
    depth: usize,
    items: &[EcmaItem],
) -> Result<(), EcmaError> {
    for item in items {
        write_item(writer, depth, item)?;
    }
    Ok(())
}

/// Writes one top-level item.
///
/// - `writer`: Accumulating sink.
/// - `depth`: Nesting depth of the item.
/// - `item`: Item to render.
///
/// # Returns
/// Unit once the item is written.
///
/// # Errors
/// Returns an error when the item cannot be rendered.
fn write_item(writer: &mut Writer, depth: usize, item: &EcmaItem) -> Result<(), EcmaError> {
    match item {
        EcmaItem::Import(value) => {
            writer.push_line(depth, &import_text(value, writer.style()));
            Ok(())
        }
        EcmaItem::Declaration(declaration) => write_declaration(writer, depth, "", declaration),
        EcmaItem::Exported(declaration) => write_declaration(writer, depth, "export ", declaration),
        EcmaItem::Statement(statement) => {
            crate::render::statements::write_statement(writer, depth, statement)
        }
    }
}

/// Writes one declaration with an optional `export` prefix.
///
/// - `writer`: Accumulating sink.
/// - `depth`: Nesting depth of the declaration.
/// - `prefix`: `export ` when the declaration is exported.
/// - `declaration`: Declaration to render.
///
/// # Returns
/// Unit once the declaration is written.
///
/// # Errors
/// Returns an error when the declaration cannot be rendered.
pub(crate) fn write_declaration(
    writer: &mut Writer,
    depth: usize,
    prefix: &str,
    declaration: &EcmaDeclaration,
) -> Result<(), EcmaError> {
    match declaration {
        EcmaDeclaration::Variable(value) => {
            writer.push_line(depth, &variable_text(value, prefix, writer.style())?);
            Ok(())
        }
        EcmaDeclaration::Function(value) => write_function(writer, depth, prefix, value),
        EcmaDeclaration::Class(value) => write_class(writer, depth, prefix, value),
    }
}

/// Writes one function declaration.
///
/// - `writer`: Accumulating sink.
/// - `depth`: Nesting depth of the declaration.
/// - `prefix`: `export ` when the declaration is exported.
/// - `function`: Function to render.
///
/// # Returns
/// Unit once the function is written.
///
/// # Errors
/// Returns an error when the body cannot be rendered.
fn write_function(
    writer: &mut Writer,
    depth: usize,
    prefix: &str,
    function: &EcmaFunction,
) -> Result<(), EcmaError> {
    let head = format!(
        "{prefix}function {}({}) {{",
        function.name,
        parameter_text(&function.parameters)
    );
    if function.body.is_empty() {
        writer.push_line(depth, &format!("{head}}}"));
        return Ok(());
    }
    writer.push_line(depth, &head);
    write_body(writer, depth + 1, &function.body)?;
    writer.push_line(depth, "}");
    Ok(())
}

/// Writes one class declaration.
///
/// - `writer`: Accumulating sink.
/// - `depth`: Nesting depth of the declaration.
/// - `prefix`: `export ` when the declaration is exported.
/// - `class`: Class to render.
///
/// # Returns
/// Unit once the class and its members are written.
///
/// # Errors
/// Returns an error when a member body cannot be rendered.
fn write_class(
    writer: &mut Writer,
    depth: usize,
    prefix: &str,
    class: &EcmaClass,
) -> Result<(), EcmaError> {
    let head = format!("{prefix}class {} {{", class.name);
    if class.members.is_empty() {
        writer.push_line(depth, &format!("{head}}}"));
        return Ok(());
    }
    writer.push_line(depth, &head);
    for member in &class.members {
        write_method(writer, depth + 1, member)?;
    }
    writer.push_line(depth, "}");
    Ok(())
}

/// Writes one class method member.
///
/// - `writer`: Accumulating sink.
/// - `depth`: Nesting depth of the member.
/// - `member`: Method member to render.
///
/// # Returns
/// Unit once the member is written.
///
/// # Errors
/// Returns an error when the method body cannot be rendered.
fn write_method(writer: &mut Writer, depth: usize, member: &EcmaMethod) -> Result<(), EcmaError> {
    let head = format!("{}({}) {{", member.name, parameter_text(&member.parameters));
    if member.body.is_empty() {
        writer.push_line(depth, &format!("{head}}}"));
        return Ok(());
    }
    writer.push_line(depth, &head);
    write_body(writer, depth + 1, &member.body)?;
    writer.push_line(depth, "}");
    Ok(())
}

/// Renders one `import` declaration.
///
/// - `import`: Import declaration to render.
/// - `style`: Style supplying the quote character.
///
/// # Returns
/// The single-line import statement.
fn import_text(import: &EcmaImport, style: EcmaStyle) -> String {
    let specifier = crate::render::expressions::quoted(&import.source, style);
    let mut bindings = Vec::new();
    if let Some(default) = &import.default {
        bindings.push(default.clone());
    }
    if !import.named.is_empty() {
        let named = import
            .named
            .iter()
            .map(named_text)
            .collect::<Vec<_>>()
            .join(", ");
        bindings.push(format!("{{ {named} }}"));
    }
    if bindings.is_empty() {
        format!("import {specifier};")
    } else {
        format!("import {} from {specifier};", bindings.join(", "))
    }
}

/// Renders one named import binding.
///
/// - `name`: Named binding to render.
///
/// # Returns
/// The `imported` or `imported as local` text.
fn named_text(name: &EcmaImportedName) -> String {
    match &name.local {
        Some(local) => format!("{} as {local}", name.imported),
        None => name.imported.clone(),
    }
}

/// Renders one variable declaration as a single line.
///
/// - `variable`: Variable declaration to render.
/// - `prefix`: `export ` when the declaration is exported.
/// - `style`: Style supplying indentation and quoting.
///
/// # Returns
/// The single-line declaration, terminated with a semicolon.
///
/// # Errors
/// Returns an error when an initializer cannot be rendered.
pub(crate) fn variable_text(
    variable: &EcmaVariable,
    prefix: &str,
    style: EcmaStyle,
) -> Result<String, EcmaError> {
    let keyword = match variable.kind {
        EcmaVariableKind::Var => "var",
        EcmaVariableKind::Let => "let",
        EcmaVariableKind::Const => "const",
    };
    let bindings = variable
        .bindings
        .iter()
        .map(|binding| binding_text(binding, style))
        .collect::<Result<Vec<_>, _>>()?
        .join(", ");
    Ok(format!("{prefix}{keyword} {bindings};"))
}

/// Renders one variable binding.
///
/// - `binding`: Binding to render.
/// - `style`: Style supplying indentation and quoting.
///
/// # Returns
/// The binding name with its optional initializer.
///
/// # Errors
/// Returns an error when the initializer cannot be rendered.
fn binding_text(binding: &EcmaBinding, style: EcmaStyle) -> Result<String, EcmaError> {
    match &binding.initializer {
        Some(value) => Ok(format!(
            "{} = {}",
            binding.name,
            expression_text(value, 0, style)?
        )),
        None => Ok(binding.name.clone()),
    }
}

/// Renders one parameter list.
///
/// - `parameters`: Parameters to render.
///
/// # Returns
/// The comma-separated parameter names.
pub(crate) fn parameter_text(parameters: &[EcmaParameter]) -> String {
    parameters
        .iter()
        .map(|parameter| parameter.name.clone())
        .collect::<Vec<_>>()
        .join(", ")
}
