use crate::source::RepositoryContext;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeSet {
    pub id: String,
    pub context: RepositoryContext,
}
