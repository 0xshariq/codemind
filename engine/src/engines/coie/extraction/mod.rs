pub mod evidence;
pub mod normalize;
pub mod resolve;
pub mod validate;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub name: String,
    pub line: u32,
}
