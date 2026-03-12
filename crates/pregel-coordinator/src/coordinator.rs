//! The Coordinator: central orchestrator.

use pregel_common::WorkerId;
use pregel_core::Superstep;

use crate::worker_registry::{WorkerInfo, WorkerRegistry};

/// The Pregel coordinator: tracks workers and advances supersteps.
///
/// A single coordinator runs per cluster. Workers register on startup.
/// Each superstep: workers report completion; when all have reported,
/// the coordinator advances the superstep and workers proceed.
pub struct Coordinator {
    pub workers: WorkerRegistry,
    pub current_superstep: Superstep,
}

impl Coordinator {
    pub fn new() -> Self {
        Self {
            workers: WorkerRegistry::new(),
            current_superstep: Superstep::new(0),
        }
    }

    pub fn register_worker(&mut self, worker_id: WorkerId, info: WorkerInfo) {
        self.workers.register(worker_id, info);
    }

    pub fn advance_superstep(&mut self) {
        self.current_superstep = self.current_superstep.next();
    }

    pub fn superstep(&self) -> u64 {
        self.current_superstep.step
    }
}

impl Default for Coordinator {
    fn default() -> Self {
        Self::new()
    }
}
