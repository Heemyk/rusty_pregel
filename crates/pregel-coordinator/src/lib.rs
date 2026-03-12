//! # pregel-coordinator
//!
//! The control plane: coordinates workers, barrier synchronization, and job lifecycle.

pub mod barrier;
pub mod coordinator;
pub mod grpc;
pub mod job_manager;
pub mod worker_registry;

pub use coordinator::Coordinator;
pub use job_manager::JobManager;
pub use worker_registry::{WorkerInfo, WorkerRegistry};
