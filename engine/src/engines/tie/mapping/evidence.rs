use crate::engines::coie::model::SourceEvidence;
use crate::source::RepositoryContext;
use std::path::{Component, Path};

pub fn normalize_path(path: &str) -> Option<String> {
    let normalized = path.trim().replace('\\', "/");
    if normalized.is_empty() || !is_safe_relative(&normalized) {
        return None;
    }
    Some(normalized)
}

pub fn is_safe_relative(path: &str) -> bool {
    let path = Path::new(path);
    if path.as_os_str().is_empty() || path.is_absolute() {
        return false;
    }
    !path.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    })
}

pub fn validate_evidence(
    evidence: &SourceEvidence,
    context: &RepositoryContext,
) -> Result<(), String> {
    if evidence.snapshot_id != context.source_snapshot_id {
        return Err("TIE evidence snapshot does not match relationship context".into());
    }
    if !is_safe_relative(&evidence.path) {
        return Err("TIE evidence path is not repository-relative".into());
    }
    if evidence.start_line == 0 || evidence.end_line < evidence.start_line {
        return Err("TIE evidence range is invalid".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context() -> RepositoryContext {
        RepositoryContext {
            repository_id: "repo".into(),
            workspace_id: "workspace".into(),
            source_snapshot_id: "snapshot".into(),
            graph_version: "graph".into(),
        }
    }

    #[test]
    fn evidence_normalization_rejects_parent_paths_and_snapshot_mismatch() {
        assert_eq!(
            normalize_path(r"tests\unit.py"),
            Some("tests/unit.py".into())
        );
        assert!(normalize_path("../secret.py").is_none());
        let evidence = SourceEvidence {
            path: "tests/unit.py".into(),
            start_line: 2,
            end_line: 3,
            snapshot_id: "snapshot".into(),
            provenance: "test".into(),
        };
        assert!(validate_evidence(&evidence, &context()).is_ok());
        let wrong = SourceEvidence {
            snapshot_id: "other".into(),
            ..evidence
        };
        assert!(validate_evidence(&wrong, &context()).is_err());
    }
}
