use crate::contract::events::EngineEvent;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LifecycleEvent {
    Engine(EngineEvent),
}

impl LifecycleEvent {
    pub fn metadata(&self) -> &crate::contract::events::EventMetadata {
        match self {
            Self::Engine(event) => event.metadata(),
        }
    }
}
