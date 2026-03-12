//! # pregel-messaging
//!
//! Message passing abstractions for worker-to-worker and worker-to-coordinator
//! communication.
//!
//! Defines [`MessageBatch`] (messages grouped by target worker) and
//! [`MessagePayload`] (the protocol: vertex messages vs. barrier acks).

pub mod message_batch;
pub mod protocol;

pub use message_batch::MessageBatch;
pub use protocol::MessagePayload;
