//! Serialization helpers for converting Rust types to and from bytes.
//!
//! We use [bincode](https://docs.rs/bincode), a compact binary format. It's smaller
//! than JSON and faster to parse. Use these functions when:
//!
//! * Sending data over the network (message payloads, protocol messages)
//! * Writing checkpoints to disk
//! * Storing graph data in binary form
//!
//! **Rust note:** The `T: Serialize` and `T: Deserialize<'a>` are *trait bounds*.
//! They mean "this function works with any type T that implements Serialize/Deserialize".
//! Most structs get these via `#[derive(Serialize, Deserialize)]`.

use crate::Result;
use serde::{Deserialize, Serialize};

/// Convert a Rust value into a byte vector.
///
/// The type must implement `Serialize` (usually via `#[derive(Serialize)]`).
/// Returns an error if serialization fails (e.g., type contains non-serializable data).
///
/// # Example
///
/// ```ignore
/// use pregel_common::serialization::serialize;
/// use serde::Serialize;
///
/// #[derive(Serialize)]
/// struct MyMsg { value: f64 }
///
/// let msg = MyMsg { value: 0.85 };
/// let bytes = serialize(&msg)?;
/// ```
pub fn serialize<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    bincode::serialize(value).map_err(|e| crate::PregelError::Serialization(e.to_string()))
}

/// Reconstruct a Rust value from bytes.
///
/// The type must implement `Deserialize`. The `'a` lifetime means the returned
/// value might borrow from `bytes` (for zero-copy deserialization in some formats).
/// With bincode, we usually own the data.
///
/// # Example
///
/// ```ignore
/// use pregel_common::serialization::deserialize;
///
/// let msg: MyMsg = deserialize(&bytes)?;
/// ```
pub fn deserialize<'a, T: Deserialize<'a>>(bytes: &'a [u8]) -> Result<T> {
    bincode::deserialize(bytes).map_err(|e| crate::PregelError::Serialization(e.to_string()))
}
