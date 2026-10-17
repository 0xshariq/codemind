use serde::{Deserialize, Serialize};
use std::{ffi::OsStr, path::Path};

/// Languages accepted by the initial source-analysis pipeline.
///
/// `TypeScript` and `Tsx` remain distinct because they require different
/// Tree-sitter grammars even though both use the TypeScript grammar package.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Language {
    Rust,
    JavaScript,
    TypeScript,
    Tsx,
    Python,
}

impl Language {
    /// Classify a source path by its extension without reading or executing it.
    pub fn from_path(path: &Path) -> Option<Self> {
        Self::supported_extension(path.extension()?)
    }

    /// Stable, human-readable identity used in deterministic manifest material.
    pub const fn stable_name(self) -> &'static str {
        match self {
            Self::Rust => "rust",
            Self::JavaScript => "javascript",
            Self::TypeScript => "typescript",
            Self::Tsx => "tsx",
            Self::Python => "python",
        }
    }

    /// Return the parser grammar family selected by this language.
    pub const fn grammar_name(self) -> &'static str {
        match self {
            Self::Rust => "tree-sitter-rust",
            Self::JavaScript => "tree-sitter-javascript",
            Self::TypeScript => "tree-sitter-typescript/typescript",
            Self::Tsx => "tree-sitter-typescript/tsx",
            Self::Python => "tree-sitter-python",
        }
    }

    pub fn supported_extension(extension: &OsStr) -> Option<Self> {
        let extension = extension.to_str()?;
        match extension.to_ascii_lowercase().as_str() {
            "rs" => Some(Self::Rust),
            "js" | "jsx" => Some(Self::JavaScript),
            "ts" => Some(Self::TypeScript),
            "tsx" => Some(Self::Tsx),
            "py" => Some(Self::Python),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Language;
    use std::path::Path;

    #[test]
    fn distinguishes_typescript_and_tsx_grammars() {
        assert_eq!(
            Language::from_path(Path::new("component.ts")),
            Some(Language::TypeScript)
        );
        assert_eq!(
            Language::from_path(Path::new("component.tsx")),
            Some(Language::Tsx)
        );
        assert_ne!(
            Language::TypeScript.grammar_name(),
            Language::Tsx.grammar_name()
        );
    }

    #[test]
    fn classifies_supported_extensions_case_insensitively() {
        assert_eq!(
            Language::from_path(Path::new("lib.RS")),
            Some(Language::Rust)
        );
        assert_eq!(
            Language::from_path(Path::new("app.JSX")),
            Some(Language::JavaScript)
        );
        assert_eq!(
            Language::from_path(Path::new("stub.PY")),
            Some(Language::Python)
        );
        assert_eq!(Language::from_path(Path::new("image.png")), None);
        assert_eq!(Language::from_path(Path::new("extensionless")), None);
    }
}
