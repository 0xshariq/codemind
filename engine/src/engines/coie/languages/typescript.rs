use crate::engines::coie::extraction::{Candidate, normalize::identifier};

pub fn declaration(line: &str, line_number: u32) -> Option<Candidate> {
    let name = line.trim().strip_prefix("function ")?.split('(').next()?;
    Some(Candidate {
        name: identifier(name)?,
        line: line_number,
    })
}
