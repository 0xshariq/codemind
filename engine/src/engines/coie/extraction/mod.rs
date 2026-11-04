pub mod evidence;
pub mod normalize;
pub mod resolve;
pub mod validate;

use crate::{
    engines::coie::model::ResolutionStatus,
    source::{Language, RepositoryContext, tree_sitter::SourceRange},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub name: String,
    pub path: String,
    pub language: Language,
    pub native_kind: String,
    pub line: u32,
    pub range: SourceRange,
    pub context: RepositoryContext,
    pub enclosing: Option<String>,
    pub resolution: ResolutionStatus,
}
