//! Worker registration and metadata.

use pregel_common::WorkerId;
use std::collections::HashMap;

/// Metadata about a registered worker.
#[derive(Debug, Clone)]
pub struct WorkerInfo {
    pub id: WorkerId,
    pub address: String,
    pub vertex_count: u64,
}

/// Registry of active workers.
#[derive(Debug, Default)]
pub struct WorkerRegistry {
    workers: HashMap<WorkerId, WorkerInfo>,
}

impl WorkerRegistry {
    pub fn new() -> Self {
        Self {
            workers: HashMap::new(),
        }
    }

    pub fn register(&mut self, id: WorkerId, info: WorkerInfo) {
        self.workers.insert(id, info);
    }

    pub fn get(&self, id: WorkerId) -> Option<&WorkerInfo> {
        self.workers.get(&id)
    }

    pub fn worker_count(&self) -> usize {
        self.workers.len()
    }
}
