//! Aggregators: global value reduction across the cluster.
//!
//! Sometimes you need a *global* value – e.g., total number of messages sent,
//! or the maximum vertex degree. Aggregators let each vertex contribute a value,
//! and the coordinator reduces them (sum, max, min, etc.) and broadcasts the
//! result back to all workers for the next superstep.

/// Trait for aggregating values from all vertices into a single result.
///
/// The coordinator collects values from workers, then calls `aggregate` to
/// reduce them. The result is broadcast to workers for the next superstep.
///
/// # Type Parameters
///
/// * `V` - The type each vertex contributes (e.g., `f64` for a partial sum)
/// * `R` - The type of the final result (e.g., `f64` for the global sum)
///
/// # Example Implementations
///
/// * **Sum:** `values.sum()` or manual fold
/// * **Max:** `values.max().unwrap_or(0)`
/// * **Count:** `values.count()`
///
/// # Usage
///
/// In a VertexProgram, you'd call something like `ctx.aggregate("sum", my_value)`.
/// The coordinator collects all contributions and applies the registered Aggregator.
pub trait Aggregator<V, R>: Send + Sync {
    /// Reduce an iterator of values into a single result.
    fn aggregate(&self, values: impl Iterator<Item = V>) -> R;
}
