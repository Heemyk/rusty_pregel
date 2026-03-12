//! # pregel-core
//!
//! Core execution abstractions for the Pregel BSP (Bulk Synchronous Parallel) model.
//!
//! This crate defines:
//! * **Superstep** – the unit of synchronous computation
//! * **Partitioning** – which worker owns which vertices
//! * **Runtime configuration** – worker count, checkpointing
//!
//! Used by both the coordinator and workers.

pub mod aggregator;
pub mod algo;
pub mod partition;
pub mod runtime;
pub mod scheduler;
pub mod superstep;

pub use algo::Algorithm;
pub use partition::{
    partition, load_strategy, CustomPartition, HashPartition, PartitionMetadata, PartitionStrategy,
    PartitionStrategyImpl,
};
pub use runtime::RuntimeConfig;
pub use superstep::Superstep;
