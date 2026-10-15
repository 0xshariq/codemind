use super::operation::Operation;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Progress {
    pub sequence: u64,
    pub engine: String,
    pub stage: String,
    pub message: String,
}

impl Progress {
    pub fn new(
        sequence: u64,
        engine: impl Into<String>,
        stage: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            sequence,
            engine: engine.into(),
            stage: stage.into(),
            message: message.into(),
        }
    }
}

pub fn investigation_stages(_operation: &Operation) -> [Progress; 4] {
    [
        Progress::new(1, "agent", "planning", "Planning investigation"),
        Progress::new(2, "coie", "coie", "Collecting code evidence"),
        Progress::new(3, "tie", "tie", "Collecting test intelligence"),
        Progress::new(4, "chie", "chie", "Computing change impact"),
    ]
}
