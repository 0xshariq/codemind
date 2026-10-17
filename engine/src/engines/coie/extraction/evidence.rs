use crate::engines::coie::model::SourceEvidence as CanonicalSourceEvidence;

pub type SourceEvidence = CanonicalSourceEvidence;

pub fn from_location(
    path: impl Into<String>,
    line: u32,
    snapshot_id: impl Into<String>,
) -> SourceEvidence {
    SourceEvidence {
        path: path.into(),
        start_line: line,
        end_line: line,
        snapshot_id: snapshot_id.into(),
        provenance: "tree-sitter".to_owned(),
    }
}

pub fn from_range(
    path: impl Into<String>,
    start_line: u32,
    end_line: u32,
    snapshot_id: impl Into<String>,
    provenance: impl Into<String>,
) -> SourceEvidence {
    SourceEvidence {
        path: path.into(),
        start_line,
        end_line,
        snapshot_id: snapshot_id.into(),
        provenance: provenance.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evidence_helpers_preserve_snapshot_and_range() {
        let one = from_location("src/lib.rs", 4, "snapshot");
        assert_eq!(one.start_line, 4);
        assert_eq!(one.end_line, 4);
        let two = from_range("src/lib.rs", 2, 6, "snapshot", "adapter");
        assert_eq!(two.end_line, 6);
        assert_eq!(two.provenance, "adapter");
    }
}
