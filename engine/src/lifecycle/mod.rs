pub mod events;
pub mod operation;
pub mod progress;
pub mod recovery;

pub use operation::Operation;

// Level 2 lifecycle tests verify the operation state/event/recovery files as a
// single module. Persistence adapters must provide the durable backing.
