//! # pregel-worker
//!
//! The worker runtime: executes vertex compute, manages message passing, and
//! participates in barrier synchronization.
//!
//! Each worker owns a [`GraphPartition`](pregel_storage::GraphPartition) and runs
//! the BSP loop: receive messages → compute vertices → route & send messages → barrier.

pub mod execution;
pub mod messaging;
pub mod partition;
pub mod worker;

pub use worker::Worker;
