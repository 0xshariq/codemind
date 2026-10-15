use crate::engines::coie::extraction::{Candidate, normalize::identifier};

pub fn declaration(line: &str, line_number: u32) -> Option<Candidate> {
    let name = line.trim().strip_prefix("def ")?.split('(').next()?;
    Some(Candidate {
        name: identifier(name.trim())?,
        line: line_number,
    })
}
