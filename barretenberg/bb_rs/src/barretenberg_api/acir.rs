use super::{bindgen};
use crate::barretenberg_api::utils::{SerializeBuffer};
use std::env;

#[derive(Debug)]
pub struct CircuitSizes {
    pub total: u32,
    pub subgroup: u32,
}

/// Get the circuit sizes (total and subgroup) from the ACIR constraint system buffer.
pub fn get_circuit_sizes(constraint_system_buf: &[u8], has_ipa_claim: bool) -> CircuitSizes {
    let mut total: u32 = 0;
    let mut subgroup: u32 = 0;
    // if the constraint system buffer is empty, there are no gates, by definition. No need to call FFI.
    if !constraint_system_buf.is_empty() {
        // keep the Buffer alive for the FFI call
        let buffer = constraint_system_buf.to_buffer();
        unsafe {
            let buf_ptr = buffer.as_slice().as_ptr();
            let ipa_ptr = &has_ipa_claim as *const bool;
            bindgen::acir_get_circuit_sizes(buf_ptr, ipa_ptr, &mut total, &mut subgroup);
        }
        // bindgen::acir_get_circuit_sizes returns big_endian u32 values, so convert them back to native
        total = u32::from_be(total);
        subgroup = u32::from_be(subgroup);
    }
    CircuitSizes { total, subgroup }
}

pub fn acir_set_slow_low_memory(enabled: bool) {
    if enabled {
        env::set_var("BB_SLOW_LOW_MEMORY", "1");
    } else {
        env::remove_var("BB_SLOW_LOW_MEMORY");
    }
}

pub fn acir_get_slow_low_memory() -> bool {
    env::var("BB_SLOW_LOW_MEMORY").map_or(false, |val| val == "1")
}
