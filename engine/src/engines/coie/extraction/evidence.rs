#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceEvidence {
    pub path: String,
    pub line: u32,
}

pub fn from_location(path: impl Into<String>, line: u32) -> SourceEvidence {
    SourceEvidence {
        path: path.into(),
        line,
    }
}
