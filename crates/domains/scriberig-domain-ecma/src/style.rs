//! Declares the exact ECMAScript source rendering style owned by this domain.
//!
//! The style changes presentation only. Every supported value is implemented by
//! `crate::render`, so selecting one never changes the semantic source model.
//! Slice two adds the `ProjectionStyleAdapter` wrapper; the field set, canonical
//! encoding, and validation live here so the adapter only delegates.

/// Projection kinds whose source text this domain renders.
///
/// These are the adapter identifiers `scriberig-project` selects from a file
/// extension, so a style adapter registered for them covers every ECMAScript
/// source this domain can parse.
pub const ECMA_SOURCE_KINDS: &[&str] = &["javascript", "typescript"];

/// Indentation characters the ECMAScript renderer may emit.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EcmaIndent {
    /// Indent with spaces at the configured width.
    Spaces,
    /// Indent with one tab character per level.
    Tabs,
}

/// Quote characters the ECMAScript renderer may emit for string literals.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EcmaQuote {
    /// Quote string literals with `"`.
    Double,
    /// Quote string literals with `'`.
    Single,
}

/// Exact ECMAScript source rendering style owned by this domain.
///
/// # Public members
/// - `indent`: Indentation character family.
/// - `indent_width`: Space width per indentation level.
/// - `quote`: String literal quote character.
/// - `validate`: Rejects values the renderer does not implement.
/// - `encode`: Produces the canonical style bytes.
/// - `decode`: Reads canonical style bytes.
/// - `indentation`: Returns the indentation unit for one level.
/// - `quote_character`: Returns the string literal quote character.
///
/// # Private members
/// - None.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EcmaStyle {
    /// Indentation character family.
    pub indent: EcmaIndent,
    /// Space width per indentation level.
    pub indent_width: u8,
    /// String literal quote character.
    pub quote: EcmaQuote,
}

impl Default for EcmaStyle {
    /// Builds the canonical ECMAScript rendering style.
    ///
    /// # Returns
    /// The style a plain formatter emits: two-space indentation and double
    /// quotes, which is what the renderer emitted before styles existed.
    fn default() -> Self {
        Self {
            indent: EcmaIndent::Spaces,
            indent_width: 2,
            quote: EcmaQuote::Double,
        }
    }
}

impl EcmaStyle {
    /// Rejects presentation values the renderer does not implement.
    ///
    /// - `self`: Style to validate.
    ///
    /// # Returns
    /// Unit when every field is one the renderer implements.
    ///
    /// # Errors
    /// Returns the offending field name when a value is unsupported.
    pub fn validate(&self) -> Result<(), String> {
        if !matches!(self.indent_width, 2 | 4 | 8) {
            return Err("indent-width must be 2, 4, or 8".to_owned());
        }
        Ok(())
    }

    /// Returns the indentation unit for one nesting level.
    ///
    /// - `self`: Style supplying the presentation choice.
    ///
    /// # Returns
    /// The exact text inserted for one indentation level.
    #[must_use]
    pub fn indentation(&self) -> String {
        match self.indent {
            EcmaIndent::Tabs => "\t".to_owned(),
            EcmaIndent::Spaces => " ".repeat(usize::from(self.indent_width)),
        }
    }

    /// Returns the quote character for string literals.
    ///
    /// - `self`: Style supplying the presentation choice.
    ///
    /// # Returns
    /// The exact quote character emitted around string literal values.
    #[must_use]
    pub fn quote_character(&self) -> char {
        match self.quote {
            EcmaQuote::Double => '"',
            EcmaQuote::Single => '\'',
        }
    }

    /// Encodes the style into its canonical domain bytes.
    ///
    /// - `self`: Style to encode.
    ///
    /// # Returns
    /// Canonical field bytes in fixed order.
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        format!(
            "indent={},indent-width={},quote={}",
            match self.indent {
                EcmaIndent::Spaces => "spaces",
                EcmaIndent::Tabs => "tabs",
            },
            self.indent_width,
            match self.quote {
                EcmaQuote::Double => "double",
                EcmaQuote::Single => "single",
            },
        )
        .into_bytes()
    }

    /// Decodes canonical domain style bytes.
    ///
    /// - `bytes`: Exact style bytes produced by a caller or encoder.
    ///
    /// # Returns
    /// The decoded style when every field is recognized.
    ///
    /// # Errors
    /// Returns the unrecognized field or value when decoding fails.
    pub fn decode(bytes: &[u8]) -> Result<Self, String> {
        let text =
            std::str::from_utf8(bytes).map_err(|_| "style bytes are not utf-8".to_owned())?;
        let mut style = Self::default();
        if text.trim().is_empty() {
            return Ok(style);
        }
        for entry in text.split(',') {
            let (key, value) = entry
                .split_once('=')
                .ok_or_else(|| format!("style field {entry} has no value"))?;
            match key.trim() {
                "indent" => {
                    style.indent = match value.trim() {
                        "spaces" => EcmaIndent::Spaces,
                        "tabs" => EcmaIndent::Tabs,
                        other => return Err(format!("indent must be spaces or tabs, got {other}")),
                    };
                }
                "indent-width" => {
                    style.indent_width = value
                        .trim()
                        .parse::<u8>()
                        .map_err(|_| "indent-width must be a number".to_owned())?;
                }
                "quote" => {
                    style.quote = match value.trim() {
                        "double" => EcmaQuote::Double,
                        "single" => EcmaQuote::Single,
                        other => {
                            return Err(format!("quote must be double or single, got {other}"));
                        }
                    };
                }
                other => return Err(format!("unknown style field {other}")),
            }
        }
        Ok(style)
    }
}
