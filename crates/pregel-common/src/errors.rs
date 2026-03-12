//! Error handling for the Pregel framework.
//!
//! Rust uses the `Result<T, E>` type for fallible operations instead of exceptions.
//! This module defines our error type (`PregelError`) and a convenience type alias
//! (`Result<T>`) so you don't have to write `Result<T, PregelError>` everywhere.

use thiserror::Error;

/// Errors that can occur anywhere in the Pregel system.
///
/// We use an enum so different error kinds can carry different information. The
/// `#[error(...)]` attribute (from [thiserror](https://docs.rs/thiserror)) generates
/// the `Display` implementation so errors print nicely. The `#[from]` attribute
/// on `Io` means `std::io::Error` automatically converts to `PregelError::Io`.
///
/// # Variants
///
/// * `Io` - File or network I/O failed (e.g., can't read checkpoint file)
/// * `Serialization` - Failed to encode/decode data (e.g., corrupt bytes)
/// * `Network` - Connection problems between workers/coordinator
/// * `Worker` - A worker reported an error or crashed
/// * `Checkpoint` - Checkpoint save/load failed
///
/// # Example
///
/// ```ignore
/// use pregel_common::{Result, PregelError};
///
/// fn might_fail() -> Result<u32> {
///     let x = do_something()?;  // ? propagates errors up
///     Ok(x + 1)
/// }
/// ```
#[derive(Error, Debug)]
pub enum PregelError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Worker error: {0}")]
    Worker(String),

    #[error("Checkpoint error: {0}")]
    Checkpoint(String),
}

/// Shorthand for `std::result::Result<T, PregelError>`.
///
/// Use this as the return type for any function that can fail with a Pregel error.
/// The `?` operator will propagate errors up the call stack.
pub type Result<T> = std::result::Result<T, PregelError>;
