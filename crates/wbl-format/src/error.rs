/// Why a file could not be read: shown next to the file in the tree, never fixed by rewriting.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FormatError {
    /// Not TOML; the message names the line.
    #[error("{0}")]
    Syntax(String),
    #[error("written by a newer Whimbrel (format {found}; this one reads up to {supported})")]
    TooNew { found: i64, supported: i64 },
    #[error("`{0}` is missing")]
    Missing(String),
    #[error("`{field}` must be {expected}")]
    Invalid { field: String, expected: String },
    #[error("{0}")]
    Io(String),
}

impl FormatError {
    pub(crate) fn invalid(field: &str, expected: &str) -> Self {
        Self::Invalid {
            field: field.to_owned(),
            expected: expected.to_owned(),
        }
    }

    /// TomlError's text starts with "TOML parse error at line L, column C".
    pub(crate) fn syntax(error: &toml_edit::TomlError) -> Self {
        Self::Syntax(error.to_string().trim_end().to_owned())
    }
}
