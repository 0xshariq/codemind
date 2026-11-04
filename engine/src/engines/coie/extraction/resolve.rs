use super::Candidate;
use crate::engines::coie::model::ResolutionStatus;

pub fn by_name<'a>(candidates: &'a [Candidate], name: &str) -> Option<&'a Candidate> {
    let mut matches = candidates.iter().filter(|candidate| candidate.name == name);
    let first = matches.next()?;
    matches.next().is_none().then_some(first)
}

pub fn in_path<'a>(candidates: &'a [Candidate], path: &str, name: &str) -> Vec<&'a Candidate> {
    candidates
        .iter()
        .filter(|c| c.path == path && c.name == name)
        .collect()
}

pub fn in_scope<'a>(
    candidates: &'a [Candidate],
    path: &str,
    enclosing: Option<&str>,
    name: &str,
) -> Vec<&'a Candidate> {
    candidates
        .iter()
        .filter(|c| c.path == path && c.enclosing.as_deref() == enclosing && c.name == name)
        .collect()
}

pub fn status(candidates: &[Candidate], name: &str) -> ResolutionStatus {
    match candidates
        .iter()
        .filter(|candidate| candidate.name == name)
        .count()
    {
        0 => ResolutionStatus::Unresolved,
        1 => ResolutionStatus::Resolved,
        _ => ResolutionStatus::Ambiguous,
    }
}

pub fn status_in_scope(
    candidates: &[Candidate],
    path: &str,
    enclosing: Option<&str>,
    name: &str,
) -> ResolutionStatus {
    match in_scope(candidates, path, enclosing, name).len() {
        0 => ResolutionStatus::Unresolved,
        1 => ResolutionStatus::Resolved,
        _ => ResolutionStatus::Ambiguous,
    }
}
