use super::Candidate;
use crate::{engines::coie::model::ResolutionStatus, source::limits};
use std::path::Path;

pub fn supported(candidate: &Candidate) -> bool {
    !candidate.name.trim().is_empty()
        && limits::is_safe_relative(Path::new(&candidate.path))
        && !candidate.native_kind.trim().is_empty()
        && candidate.line > 0
        && candidate.range.start_line > 0
        && candidate.range.end_line >= candidate.range.start_line
        && candidate.range.start_byte <= candidate.range.end_byte
        && candidate.context.validate().is_ok()
        && !matches!(candidate.resolution, ResolutionStatus::Unsupported)
}
