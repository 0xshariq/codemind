use super::Candidate;

pub fn supported(candidate: &Candidate) -> bool {
    !candidate.name.is_empty() && candidate.line > 0
}
