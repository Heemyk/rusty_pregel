//! Helper to export a VertexProgram as a WASM `compute` function.
//!
//! Add to your crate (built with `--target wasm32-unknown-unknown`):
//!
//! ```ignore
//! use pregel_sdk::export_wasm_compute;
//! struct MyAlgo;
//! impl VertexProgram for MyAlgo { ... }
//! export_wasm_compute!(MyAlgo);
//! ```
//!
//! The macro expands to a `#[no_mangle] pub extern "C" fn compute(...)` that
//! deserializes `ComputeInput`, calls `vertex_program_compute`, serializes `ComputeResultWire`.
//! Error codes follow docs/ABI_SPEC.md §2.2.

/// Export a VertexProgram as the WASM ABI `compute` function.
///
/// Requires `P: VertexProgram + Default`. Returns output length or AbiErrorCode.
#[macro_export]
macro_rules! export_wasm_compute {
    ($program:ty) => {
        #[no_mangle]
        pub extern "C" fn compute(
            input_ptr: *const u8,
            input_len: i32,
            output_ptr: *mut u8,
            output_max_len: i32,
        ) -> i32 {
            use pregel_common::{AbiErrorCode, ComputeInput, ComputeResultWire};
            use pregel_sdk::vertex_program_compute;

            if input_len <= 0 || output_max_len <= 0 {
                return AbiErrorCode::Invalid.as_i32();
            }
            let input_slice = unsafe {
                std::slice::from_raw_parts(input_ptr as *const u8, input_len as usize)
            };
            let input: ComputeInput = match bincode::deserialize(input_slice) {
                Ok(i) => i,
                Err(_) => return AbiErrorCode::Deserialize.as_i32(),
            };
            if let Err(_) = input.validate() {
                return AbiErrorCode::Invalid.as_i32();
            }
            let superstep = input.superstep;
            let mut prog: $program = Default::default();
            let result = vertex_program_compute(&mut prog, &input, superstep);
            if let Err(_) = result.validate() {
                return AbiErrorCode::Serialize.as_i32();
            }
            let serialized = match bincode::serialize(&result) {
                Ok(s) => s,
                Err(_) => return AbiErrorCode::Serialize.as_i32(),
            };
            if serialized.len() > output_max_len as usize {
                return AbiErrorCode::OutputOverrun.as_i32();
            }
            unsafe {
                std::ptr::copy_nonoverlapping(
                    serialized.as_ptr(),
                    output_ptr as *mut u8,
                    serialized.len(),
                );
            }
            serialized.len() as i32
        }
    };
}
