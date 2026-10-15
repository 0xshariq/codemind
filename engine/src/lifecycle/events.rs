use super::progress::Progress;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LifecycleEvent {
    Progress(Progress),
    GraphUpdated,
    Completed,
    Failed(String),
}
