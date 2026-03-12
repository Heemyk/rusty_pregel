//! Aggregator values: global reduction results from the coordinator.
//!
//! When vertices use aggregators (e.g., `ctx.aggregate("sum", value)`), the
//! coordinator collects, reduces, and broadcasts. This struct holds the broadcast
//! result – a map of aggregate name → serialized value.

use std::collections::HashMap;

/// Holds the results of aggregators, distributed to workers each superstep.
///
/// Workers receive this from the coordinator after the barrier. The values
/// are serialized (as `Vec<u8>`) because different aggregators produce different
/// types (f64, u64, etc.). The vertex program deserializes as needed.
///
/// # Example
///
/// After a "sum" aggregator: `values.get("total_messages")` might return
/// the bytes of a `u64` representing the total message count.
#[derive(Debug, Clone, Default)]
pub struct AggregatorValues {
    pub values: HashMap<String, Vec<u8>>,
}

impl AggregatorValues {
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
        }
    }

    /// Store an aggregate result by name.
    pub fn set(&mut self, name: &str, value: Vec<u8>) {
        self.values.insert(name.to_string(), value);
    }

    /// Retrieve an aggregate result by name.
    pub fn get(&self, name: &str) -> Option<&[u8]> {
        self.values.get(name).map(|v| v.as_slice())
    }
}
