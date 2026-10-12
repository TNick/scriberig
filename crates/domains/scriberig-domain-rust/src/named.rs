//! Extracts stable, human-readable Rust symbols for semantic navigation.

use quote::ToTokens;
use syn::Item;

/// A named Rust symbol with a stable semantic key and source range.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NamedRustSymbol {
    /// Canonical semantic kind.
    pub kind: String,
    /// Human-readable declaration name.
    pub name: String,
    /// Hierarchical identity key within one source file.
    pub key: String,
    /// Approximate byte range covering the declaration name.
    pub range: (usize, usize),
}

/// Extracts named modules, types, functions, and methods from Rust source.
///
/// # Parameters
/// - `source`: Rust source text to parse.
///
/// # Returns
/// Named symbols in source order.
///
/// # Errors
/// Returns the parser diagnostic when the source is not valid Rust.
pub fn named_symbols(source: &str) -> Result<Vec<NamedRustSymbol>, String> {
    let file = syn::parse_file(source).map_err(|error| error.to_string())?;
    let mut symbols = Vec::new();
    collect_items(source, &file.items, "", &mut symbols);
    Ok(symbols)
}

/// Walks a list of Rust items and records named declarations.
///
/// # Parameters
/// - `source`: Source text used for ranges.
/// - `items`: Rust items to walk.
/// - `prefix`: Parent semantic key.
/// - `symbols`: Output symbol collection.
fn collect_items(source: &str, items: &[Item], prefix: &str, symbols: &mut Vec<NamedRustSymbol>) {
    for item in items {
        match item {
            Item::Mod(module) => {
                let name = module.ident.to_string();
                let key = join_key(prefix, &name);
                push_symbol(source, "rust.module", &name, &key, symbols);
                if let Some((_, children)) = &module.content {
                    collect_items(source, children, &key, symbols);
                }
            }
            Item::Struct(value) => {
                let name = value.ident.to_string();
                let key = join_key(prefix, &name);
                push_symbol(source, "rust.struct", &name, &key, symbols);
            }
            Item::Enum(value) => {
                let name = value.ident.to_string();
                let key = join_key(prefix, &name);
                push_symbol(source, "rust.enum", &name, &key, symbols);
            }
            Item::Fn(value) => {
                let name = value.sig.ident.to_string();
                let key = join_key(prefix, &name);
                push_symbol(source, "rust.function", &name, &key, symbols);
            }
            Item::Impl(value) => {
                let owner = value.self_ty.to_token_stream().to_string();
                for item in &value.items {
                    if let syn::ImplItem::Fn(function) = item {
                        let name = function.sig.ident.to_string();
                        let key = format!("{}::{}", join_key(prefix, &owner), name);
                        push_symbol(source, "rust.method", &name, &key, symbols);
                    }
                }
            }
            _ => {}
        }
    }
}

/// Appends a symbol with a deterministic name range.
///
/// # Parameters
/// - `source`: Source text used for ranges.
/// - `kind`: Semantic kind label.
/// - `name`: Human-readable declaration name.
/// - `key`: Hierarchical semantic key.
/// - `symbols`: Output symbol collection.
fn push_symbol(
    source: &str,
    kind: &str,
    name: &str,
    key: &str,
    symbols: &mut Vec<NamedRustSymbol>,
) {
    let start = source.find(name).unwrap_or(0);
    let key = unique_key(key, symbols);
    symbols.push(NamedRustSymbol {
        kind: kind.to_owned(),
        name: name.to_owned(),
        key,
        range: (start, start.saturating_add(name.len())),
    });
}

/// Makes a repeated semantic key unique within one source file.
///
/// # Parameters
/// - `key`: Preferred semantic key.
/// - `symbols`: Symbols already extracted from the source file.
///
/// # Returns
/// The preferred key or a deterministic occurrence-suffixed variant.
fn unique_key(key: &str, symbols: &[NamedRustSymbol]) -> String {
    if !symbols.iter().any(|symbol| symbol.key == key) {
        return key.to_owned();
    }
    let mut occurrence = 2;
    loop {
        let candidate = format!("{key}#{occurrence}");
        if !symbols.iter().any(|symbol| symbol.key == candidate) {
            return candidate;
        }
        occurrence += 1;
    }
}

/// Joins one semantic path component.
///
/// # Parameters
/// - `prefix`: Existing semantic path.
/// - `name`: New path component.
///
/// # Returns
/// The joined semantic path.
fn join_key(prefix: &str, name: &str) -> String {
    if prefix.is_empty() {
        name.to_owned()
    } else {
        format!("{prefix}::{name}")
    }
}

#[cfg(test)]
mod tests {
    use super::named_symbols;

    /// Extracts the stable names used by the golden Rust fixture.
    #[test]
    fn extracts_nested_named_symbols() {
        let source = "pub mod math { pub struct Calculator; impl Calculator { pub fn add(&self, left: i32, right: i32) -> i32 { left + right } } }";
        let symbols = named_symbols(source).expect("valid Rust");
        assert!(symbols.iter().any(|value| value.name == "math"));
        assert!(symbols.iter().any(|value| value.name == "Calculator"));
        assert!(symbols.iter().any(|value| value.name == "add"));
    }

    /// Disambiguates repeated semantic declarations without dropping either one.
    #[test]
    fn disambiguates_repeated_named_symbols() {
        let source = "struct Calculator; impl Calculator { fn add(&self) {} } impl Calculator { fn add(&self) {} }";
        let symbols = named_symbols(source).expect("valid Rust");
        let methods = symbols
            .iter()
            .filter(|value| value.name == "add")
            .collect::<Vec<_>>();
        assert_eq!(methods.len(), 2);
        assert_eq!(methods[0].key, "Calculator::add");
        assert_eq!(methods[1].key, "Calculator::add#2");
    }
}
