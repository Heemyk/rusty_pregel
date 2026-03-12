//! The [`Message`] trait: marker for types that can be sent between vertices.
//!
//! This is a "marker trait" – it has no methods. Its purpose is to constrain
//! which types can be used as `VertexProgram::Message`. We require `Send + Sync + Clone`
//! because messages cross thread boundaries and may be duplicated when routing.

/// Marker trait for types that can be sent as messages between vertices.
///
/// A type implements `Message` if it is `Send`, `Sync`, and `Clone`. The blanket
/// implementation below means *any* such type automatically implements `Message`.
///
/// * **Send** – can be transferred across threads (workers run in parallel)
/// * **Sync** – can be shared across threads via references
/// * **Clone** – may need to be duplicated when sending to multiple targets
///
/// # What implements Message
///
/// Primitives: `f64`, `u64`, `i32`, etc.
/// Tuples of Messages: `(f64, u64)`
/// Structs with `#[derive(Clone)]` where all fields are `Send + Sync`
pub trait Message: Send + Sync + Clone {}

impl<T: Send + Sync + Clone> Message for T {}
