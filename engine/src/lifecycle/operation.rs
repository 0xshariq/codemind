use crate::source::RepositoryContext;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Operation {
    pub operation_id: String,
    pub context: RepositoryContext,
}

impl Operation {
    pub fn new(operation_id: impl Into<String>, context: RepositoryContext) -> Self {
        Self {
            operation_id: operation_id.into(),
            context,
        }
    }
}
