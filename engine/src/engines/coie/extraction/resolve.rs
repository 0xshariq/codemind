use super::Candidate;

pub fn by_name<'a>(candidates: &'a [Candidate], name: &str) -> Option<&'a Candidate> {
    candidates.iter().find(|candidate| candidate.name == name)
}
