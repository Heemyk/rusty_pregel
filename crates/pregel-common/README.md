# pregel-common

**Purpose:** The shared foundation crate used by every other crate in the Pregel workspace. It defines types, errors, configuration, and serialization utilities that the entire system relies on.

**Why it exists:** In a multi-crate workspace, you want to avoid duplicating definitions. If the worker and coordinator both need to know what a `Message` looks like, they both depend on `pregel-common` and use the same definition. This prevents bugs from type mismatches and keeps the codebase DRY (Don't Repeat Yourself).

## What's Inside

| Module | Purpose |
|--------|---------|
| `types` | Core type aliases and structs: `VertexId`, `WorkerId`, `Message` |
| `errors` | Error handling: `PregelError` enum and `Result` type alias |
| `config` | Configuration structs for workers and jobs |
| `serialization` | Helpers to convert Rust types to/from bytes (for network or disk) |

## Key Concepts for Rust Beginners

### Type Aliases
```rust
pub type VertexId = u64;  // Just a nickname - VertexId IS u64
pub type WorkerId = u32;  // Makes code more readable: partition(vertex, workers) vs partition(u64, usize)
```

### The `Result` Type
Rust doesn't have exceptions. Functions that can fail return `Result<T, E>`:
- `Ok(value)` = success, here's your data
- `Err(error)` = failure, here's what went wrong

We define `Result<T>` as shorthand for `Result<T, PregelError>` so you don't have to type the error type everywhere.

### Serde
`Serialize` and `Deserialize` are traits from the [serde](https://serde.rs/) crate. Adding `#[derive(Serialize, Deserialize)]` to a struct lets you convert it to JSON, bincode (binary), etc. We use bincode for compact binary serialization over the network and for checkpoints.
