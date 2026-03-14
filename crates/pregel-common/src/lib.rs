//! # pregel-common
//!
//! Shared types, errors, configuration, and serialization utilities for the Pregel
//! distributed graph processing framework.
//!
//! This crate is the foundation of the workspace. Every other crate depends on it
//! to ensure consistent definitions across the system (e.g., what a `Message` looks
//! like, how errors are represented).

pub mod config;
pub mod errors;
pub mod serialization;
pub mod types;

pub use config::PartitionConfig;
pub use errors::{PregelError, Result};
pub use types::{AbiErrorCode, ComputeInput, ComputeResultWire, Message, VertexId, WorkerId};
