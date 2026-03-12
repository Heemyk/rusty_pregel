//! # pregel-checkpoint
//!
//! Fault tolerance via periodic snapshots of worker state.
//!
//! Workers checkpoint every N supersteps. On failure, new workers restore from
//! the last checkpoint and resume execution.

pub mod checkpoint_manager;
pub mod recovery;
pub mod snapshot;

pub use checkpoint_manager::CheckpointManager;
pub use recovery::Recovery;
pub use snapshot::{Checkpoint, VertexState};
