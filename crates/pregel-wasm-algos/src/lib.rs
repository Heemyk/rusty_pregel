//! WASM vertex compute modules for Pregel.
//!
//! Build with: cargo build -p pregel-wasm-algos --target wasm32-unknown-unknown --release
//!
//! Exports `compute(input_ptr, input_len, output_ptr, output_max_len) -> output_len`.
//! Input: bincode ComputeInput. Output: bincode ComputeResultWire.

use pregel_common::{ComputeInput, ComputeResultWire};
use std::collections::HashSet;

/// Compute export for Connected Components. Must match host ABI.
/// Returns output length, or negative on error: -1 bad args, -2 deserialize fail, -3 serialize fail, -4 output too large.
#[no_mangle]
pub extern "C" fn compute(
    input_ptr: *const u8,
    input_len: i32,
    output_ptr: *mut u8,
    output_max_len: i32,
) -> i32 {
    // In WASM, 0 is a valid memory offset (start of linear memory), not null.
    if input_len <= 0 || output_max_len <= 0 {
        return -1;
    }
    let input_slice = unsafe { std::slice::from_raw_parts(input_ptr as *const u8, input_len as usize) };
    let input: ComputeInput = match bincode::deserialize(input_slice) {
        Ok(i) => i,
        Err(_) => return -2,
    };
    let result = cc_compute(&input);
    let serialized = match bincode::serialize(&result) {
        Ok(s) => s,
        Err(_) => return -3,
    };
    if serialized.len() > output_max_len as usize {
        return -4;
    }
    unsafe {
        std::ptr::copy_nonoverlapping(serialized.as_ptr(), output_ptr as *mut u8, serialized.len());
    }
    serialized.len() as i32
}

fn cc_compute(input: &ComputeInput) -> ComputeResultWire {
    let current = input
        .value
        .first()
        .and_then(|_| bincode::deserialize::<u64>(&input.value).ok())
        .unwrap_or(input.vertex_id);

    let messages_parsed: Vec<(u64, u64)> = input
        .messages
        .iter()
        .filter_map(|(src, p)| bincode::deserialize::<u64>(p).ok().map(|v| (*src, v)))
        .collect();

    let min_received = messages_parsed.iter().map(|(_, v)| *v).min().unwrap_or(current);
    let new_component = current.min(min_received);

    let mut neighbors: HashSet<u64> = input.edges.iter().copied().collect();
    for (src, _) in &messages_parsed {
        neighbors.insert(*src);
    }

    if messages_parsed.is_empty() {
        let payload = bincode::serialize(&new_component).unwrap();
        let outgoing: Vec<(u64, Vec<u8>)> =
            input.edges.iter().map(|&t| (t, payload.clone())).collect();
        return ComputeResultWire {
            new_value: Some(payload),
            outgoing,
        };
    }

    let any_sender_larger = messages_parsed.iter().any(|(_, v)| *v > current);
    let should_send = new_component < current || any_sender_larger;
    if !should_send {
        return ComputeResultWire { new_value: None, outgoing: vec![] };
    }

    let payload = bincode::serialize(&new_component).unwrap();
    let targets: HashSet<u64> = if new_component < current {
        neighbors
    } else {
        messages_parsed.iter().filter(|(_, v)| *v > current).map(|(src, _)| *src).collect()
    };
    let outgoing: Vec<(u64, Vec<u8>)> = targets.into_iter().map(|t| (t, payload.clone())).collect();
    ComputeResultWire {
        new_value: Some(payload),
        outgoing,
    }
}
