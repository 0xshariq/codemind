pub mod javascript;
pub mod python;
pub mod rust;
pub mod typescript;

use crate::source::Language;

pub fn supports(language: Language) -> bool {
    matches!(
        language,
        Language::Rust | Language::JavaScript | Language::TypeScript | Language::Python
    )
}
