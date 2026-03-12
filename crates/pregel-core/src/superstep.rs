//! The superstep: the unit of synchronous computation in BSP.
//!
//! In Pregel, time advances in discrete supersteps. Superstep 0 is the initial
//! state. Each superstep: receive messages → compute → send messages → barrier.

/// Represents the current superstep number.
///
/// The coordinator and workers advance this in lockstep. When all workers
/// report completion, the coordinator increments and broadcasts the new superstep.
///
/// # BSP Semantics
///
/// * Superstep 0: Vertices have initial values, no messages yet
/// * Superstep 1: First round of messages delivered, compute runs
/// * Superstep N: Messages from superstep N-1 are available
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Superstep {
    pub step: u64,
}

impl Superstep {
    /// Create a superstep with the given step number.
    pub fn new(step: u64) -> Self {
        Self { step }
    }

    /// Return the next superstep (step + 1).
    pub fn next(&self) -> Self {
        Self { step: self.step + 1 }
    }
}
