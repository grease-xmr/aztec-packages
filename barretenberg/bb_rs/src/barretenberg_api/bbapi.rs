use super::bindgen;
use crate::noir_api::artifacts::{load_binary, save_binary};
use log::*;
use num_bigint::BigUint;
use rmp_serde::{decode, encode};
use serde::{Deserialize, Serialize};
use std::os::raw::c_void;
use std::path::Path;
use std::ptr;
use std::ptr::null;
// This is not used for now, but may replace the acir functions later

#[derive(Debug, Serialize, Deserialize)]
pub struct BbErrorResponse {
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CircuitInputNoVK {
    pub name: String,
    #[serde(with = "serde_bytes")]
    pub bytecode: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CircuitInput {
    pub name: String,
    #[serde(with = "serde_bytes")]
    pub bytecode: Vec<u8>,
    #[serde(with = "serde_bytes")]
    pub verification_key: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProofSystemSettings {
    #[serde(default)]
    pub ipa_accumulation: bool,
    #[serde(default = "default_oracle_hash_type")]
    pub oracle_hash_type: String,
    #[serde(default)]
    pub disable_zk: bool,
    #[serde(default)]
    pub optimized_solidity_verifier: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Uint256(#[serde(with = "serde_bytes")] [u8; 32]);

impl Uint256 {
    pub fn as_bigint(&self) -> BigUint {
        BigUint::from_bytes_be(&self.0)
    }
}

fn default_oracle_hash_type() -> String {
    "poseidon2".to_string()
}

impl Default for ProofSystemSettings {
    fn default() -> Self {
        Self {
            ipa_accumulation: false,
            oracle_hash_type: "poseidon2".to_string(),
            disable_zk: false,
            optimized_solidity_verifier: false,
        }
    }
}

// Command structs
#[derive(Debug, Serialize, Deserialize)]
pub struct CircuitProve {
    pub circuit: CircuitInput,
    #[serde(with = "serde_bytes")]
    pub witness: Vec<u8>,
    pub settings: ProofSystemSettings,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CircuitProveResponse {
    pub public_inputs: Vec<Uint256>,
    pub proof: Vec<Uint256>,
    pub vk: CircuitComputeVkResponse,
}

impl CircuitProveResponse {
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), BbApiError> {
        let bytes = rmp_serde::to_vec_named(self)?;
        // Will automatically compress if path ends with .gz
        save_binary(path, &bytes)?;
        Ok(())
    }

    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, BbApiError> {
        let bytes = load_binary(path)?;
        let response: CircuitProveResponse = rmp_serde::from_slice(&bytes)?;
        Ok(response)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CircuitVerify {
    #[serde(with = "serde_bytes")]
    pub verification_key: Vec<u8>,
    pub public_inputs: Vec<Uint256>,
    pub proof: Vec<Uint256>,
    pub settings: ProofSystemSettings,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CircuitVerifyResponse {
    pub verified: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CircuitComputeVk {
    pub circuit: CircuitInputNoVK,
    pub settings: ProofSystemSettings,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CircuitComputeVkResponse {
    #[serde(with = "serde_bytes")]
    pub bytes: Vec<u8>,
    pub fields: Vec<Uint256>,
    #[serde(with = "serde_bytes")]
    pub hash: Vec<u8>,
}

// Error handling
#[derive(Debug, thiserror::Error)]
pub enum BbApiError {
    #[error("Msgpack encode error: {0}")]
    EncodeError(#[from] encode::Error),
    #[error("Msgpack decode error: {0}")]
    DecodeError(#[from] decode::Error),
    #[error("Invalid response: expected {expected}, got {actual}")]
    InvalidResponse { expected: String, actual: String },
    #[error("API error: {0}")]
    ApiError(String),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

impl From<BbApiError> for String {
    fn from(error: BbApiError) -> Self {
        match error {
            BbApiError::EncodeError(e) => e.to_string(),
            BbApiError::DecodeError(e) => e.to_string(),
            BbApiError::InvalidResponse { expected, actual } => {
                format!("Invalid response: expected {}, got {}", expected, actual)
            }
            BbApiError::ApiError(e) => e,
            BbApiError::IoError(e) => e.to_string(),
        }
    }
}

impl std::str::FromStr for BbApiError {
    type Err = BbApiError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Err(BbApiError::ApiError(s.to_string()))
    }
}

// Main bbapi function that handles msgpack encoding/decoding
pub fn bbapi_command<T, R>(command_name: &str, command_data: &T) -> Result<R, BbApiError>
where
    T: Serialize,
    R: for<'de> Deserialize<'de>,
{
    let encoded_command = rmp_serde::to_vec_named(&((command_name, command_data),))?;
    trace!("Encoded command: {}", hex::encode(&encoded_command));
    // Call the C++ bbapi function
    let (is_msgpack, response_bytes) = execute_bb_msgpack_command(&encoded_command);
    trace!(
        "Command result (MsgPacked): {}",
        hex::encode(&response_bytes)
    );

    if !is_msgpack {
        let response_str = String::from_utf8_lossy(&response_bytes);
        return Err(BbApiError::ApiError(format!(
            "BbApi command {command_name} failed. The C++ library says: {response_str}"
        )));
    }
    // From this point, the contract is that the data should be msgpack encoded.
    // First decode response as the expected response type, R
    let (response_name, response_data): (String, R) =
        decode::from_slice(&response_bytes).map_err(|_| {
            // Ok, it wasn't R, so it should be an ErrorResponse type
            match decode::from_slice::<(String, BbErrorResponse)>(&response_bytes) {
                // Bbapi returned "successfully" with an error response
                Ok((name, err)) if name == "ErrorResponse" => BbApiError::ApiError(format!(
                    "BbApi command {command_name} failed. The C++ library says: {}",
                    err.message
                )),
                // We should really never see this, but for completeness
                Ok((other, err)) => BbApiError::InvalidResponse {
                    expected: "ErrorResponse".into(),
                    actual: format!("{other}: {}", err.message),
                },
                // All right, there's a bug. The response is neither R nor ErrorResponse
                Err(e) => BbApiError::InvalidResponse {
                    expected: format!("{command_name}Response or ErrorResponse"),
                    actual: format!(
                        "{e}. The data returned was: {}",
                        String::from_utf8_lossy(&response_bytes)
                    ),
                },
            }
        })?;

    trace!("Decoded response: {response_name}");
    // Verify response type matches what we expect
    let expected_response = format!("{}Response", command_name);
    if response_name != expected_response {
        return Err(BbApiError::InvalidResponse {
            expected: expected_response,
            actual: response_name,
        });
    }
    Ok(response_data)
}

/// Low-level bbapi function that directly calls the C++ binding
///
/// # Safety
///
/// This function is follows the "acquire-copy-release" pattern:
///
/// *Acquire*: bindgen::bbapi allocates memory and gives a pointer.
/// *Copy*: std::slice::from_raw_parts(...).to_vec() allocates a new Vec<u8> in the Rust with a copy of the data.
/// *Release*: `bindgen::bbapi_free_result(...)` immediately frees the original C-allocated memory.
///
/// ## Safety Guarantees and Required Assumptions
/// We make the following assumptions about the bbapi library and its usage to ensure safety:
///
/// * Pointer Validity: If `out_ptr` is not null, it points to a valid, readable memory block of at least `out_len`
/// bytes.
/// * Length Correctness: out_len must accurately reflect the size of the valid memory block.
/// * Ownership: The memory pointed to by out_ptr must be exclusively owned by us and must be safe to free with
///   free_result.
/// * Compatibility: free_result must be the correct and only function to deallocate memory returned by bindgen::bbapi.
/// * Thread Safety: The call to bindgen::bbapi must be thread-safe. If it relies on static or global state without
/// synchronization, calling it from multiple threads could cause data races.
/// * No Panics: The C code must not panic or unwind across the FFI boundary.
fn execute_bb_msgpack_command(command: &[u8]) -> (bool, Vec<u8>) {
    unsafe {
        let mut out_ptr: *mut u8 = ptr::null_mut();
        let mut out_len: usize = 0;

        bindgen::bbapi_set_verbose_logging(true);
        // Definitely don't do this every time. TODO - load CRS once.
        if !bindgen::bbapi_init(null()) {
            panic!("Failed to initialize bbapi");
        }
        // Call the C++ bbapi function with all 4 required parameters
        let is_msgpack = bindgen::bbapi_non_chonk(
            command.as_ptr(), // input buffer
            command.len(),    // input length
            &mut out_ptr,     // output buffer pointer
            &mut out_len,     // output length pointer
        );

        // Convert the output to a Vec<u8>
        if out_ptr.is_null() || out_len == 0 {
            (false, b"Empty response from bbapi".to_vec())
        } else {
            let result = std::slice::from_raw_parts(out_ptr, out_len).to_vec();
            // Free the C-allocated memory immediately. We have a copy of the data in our Vec.
            bindgen::bbapi_free_result(out_ptr as *mut c_void);
            (is_msgpack, result)
        }
    }
}

// High-level API functions using the new command-based approach

/// Generate a proof using the bbapi command system
pub fn prove_ultra_honk(
    constraint_system_buf: &[u8],
    witness_buf: &[u8],
    vkey_buf: &[u8],
) -> Result<CircuitProveResponse, BbApiError> {
    let settings = ProofSystemSettings {
        ipa_accumulation: false,
        oracle_hash_type: "poseidon2".to_string(),
        disable_zk: false,
        optimized_solidity_verifier: false,
    };

    let command = CircuitProve {
        circuit: CircuitInput {
            name: "circuit".to_string(),
            bytecode: constraint_system_buf.to_vec(),
            verification_key: vkey_buf.to_vec(),
        },
        witness: witness_buf.to_vec(),
        settings,
    };

    info!("Executing UltraHonk prover");
    let response = bbapi_command::<CircuitProve, CircuitProveResponse>("CircuitProve", &command)?;
    info!("UltraHonk prover returned successfully");
    Ok(response)
}

/// Generate a proof using Keccak for EVM verification
pub fn prove_ultra_keccak_honk(
    constraint_system_buf: &[u8],
    witness_buf: &[u8],
    vkey_buf: &[u8],
) -> Result<CircuitProveResponse, BbApiError> {
    let settings = ProofSystemSettings {
        ipa_accumulation: false,
        oracle_hash_type: "keccak".to_string(),
        disable_zk: true,
        optimized_solidity_verifier: false,
    };

    let command = CircuitProve {
        circuit: CircuitInput {
            name: "circuit".to_string(),
            bytecode: constraint_system_buf.to_vec(),
            verification_key: vkey_buf.to_vec(),
        },
        witness: witness_buf.to_vec(),
        settings,
    };

    info!("Executing Barretenberg UltraHonk-NonZK prover (Keccak)");
    let response = bbapi_command::<CircuitProve, CircuitProveResponse>("CircuitProve", &command)?;
    info!("UltraHonk-NonZK prover (Keccak) completed successfully");
    Ok(response)
}

/// Generate a proof using Keccak with ZK enabled
pub fn prove_ultra_keccak_zk_honk(
    constraint_system_buf: &[u8],
    witness_buf: &[u8],
    vkey_buf: &[u8],
) -> Result<CircuitProveResponse, BbApiError> {
    let settings = ProofSystemSettings {
        ipa_accumulation: false,
        oracle_hash_type: "keccak".to_string(),
        disable_zk: false,
        optimized_solidity_verifier: false,
    };

    let command = CircuitProve {
        circuit: CircuitInput {
            name: "circuit".to_string(),
            bytecode: constraint_system_buf.to_vec(),
            verification_key: vkey_buf.to_vec(),
        },
        witness: witness_buf.to_vec(),
        settings,
    };

    info!("Executing Barretenberg UltraHonk-ZK prover (Keccak)");
    let response = bbapi_command::<CircuitProve, CircuitProveResponse>("CircuitProve", &command)?;
    info!("UltraHonk-ZK prover (Keccak) completed successfully");
    Ok(response)
}

/// Compute verification key
pub fn get_ultra_honk_verification_key(
    constraint_system_buf: &[u8],
) -> Result<CircuitComputeVkResponse, BbApiError> {
    let settings = ProofSystemSettings {
        ipa_accumulation: false,
        oracle_hash_type: "poseidon2".to_string(),
        disable_zk: false,
        optimized_solidity_verifier: false,
    };

    let command = CircuitComputeVk {
        circuit: CircuitInputNoVK {
            name: "circuit".to_string(),
            bytecode: constraint_system_buf.to_vec(),
        },
        settings,
    };

    let response =
        bbapi_command::<CircuitComputeVk, CircuitComputeVkResponse>("CircuitComputeVk", &command)?;
    Ok(response)
}

/// Compute verification key for Keccak
pub fn get_ultra_honk_keccak_verification_key(
    constraint_system_buf: &[u8],
) -> Result<Vec<u8>, BbApiError> {
    let settings = ProofSystemSettings {
        ipa_accumulation: false,
        oracle_hash_type: "keccak".to_string(),
        disable_zk: true,
        optimized_solidity_verifier: false,
    };

    let command = CircuitComputeVk {
        circuit: CircuitInputNoVK {
            name: "circuit".to_string(),
            bytecode: constraint_system_buf.to_vec(),
        },
        settings,
    };

    let response =
        bbapi_command::<CircuitComputeVk, CircuitComputeVkResponse>("CircuitComputeVk", &command)?;
    Ok(response.bytes)
}

/// Compute verification key for Keccak with ZK
pub fn get_ultra_honk_keccak_zk_verification_key(
    constraint_system_buf: &[u8],
) -> Result<Vec<u8>, BbApiError> {
    let settings = ProofSystemSettings {
        ipa_accumulation: false,
        oracle_hash_type: "keccak".to_string(),
        disable_zk: false,
        optimized_solidity_verifier: false,
    };

    let command = CircuitComputeVk {
        circuit: CircuitInputNoVK {
            name: "circuit".to_string(),
            bytecode: constraint_system_buf.to_vec(),
        },
        settings,
    };

    let response =
        bbapi_command::<CircuitComputeVk, CircuitComputeVkResponse>("CircuitComputeVk", &command)?;
    Ok(response.bytes)
}

fn to_verify(
    prf: CircuitProveResponse,
    ipa: bool,
    hash: &str,
    dzk: bool,
) -> Result<CircuitVerify, BbApiError> {
    if prf.proof.is_empty() {
        return Err(BbApiError::ApiError("Proof cannot be empty".to_string()));
    }
    let settings = ProofSystemSettings {
        ipa_accumulation: ipa,
        oracle_hash_type: hash.to_string(),
        disable_zk: dzk,
        optimized_solidity_verifier: false,
    };

    let verification_key = prf.vk.bytes;
    let public_inputs = prf.public_inputs;
    let proof = prf.proof;
    Ok(CircuitVerify {
        verification_key,
        public_inputs,
        proof,
        settings,
    })
}

/// Verify a proof
pub fn verify_ultra_honk(proof: CircuitProveResponse) -> Result<bool, BbApiError> {
    let command = to_verify(proof, false, "poseidon2", false)?;
    info!("Executing UltraHonk verifier");
    let response =
        bbapi_command::<CircuitVerify, CircuitVerifyResponse>("CircuitVerify", &command)?;
    info!(
        "UltraHonk verifier returned with result: {}",
        response.verified
    );
    Ok(response.verified)
}

/// Verify a Keccak proof
pub fn verify_ultra_keccak_honk(proof: CircuitProveResponse) -> Result<bool, BbApiError> {
    let command = to_verify(proof, false, "keccak", true)?;
    info!("Executing Keccak verifier");
    let response =
        bbapi_command::<CircuitVerify, CircuitVerifyResponse>("CircuitVerify", &command)?;
    info!(
        "Keccak verifier returned with result: {}",
        response.verified
    );
    Ok(response.verified)
}

/// Verify a Keccak ZK proof
pub fn verify_ultra_keccak_zk_honk(proof: CircuitProveResponse) -> Result<bool, BbApiError> {
    let command = to_verify(proof, false, "keccak", false)?;

    info!("Executing UltraKeccakZK verifier");
    let response =
        bbapi_command::<CircuitVerify, CircuitVerifyResponse>("CircuitVerify", &command)?;
    info!(
        "UltraKeccakZK verifier returned with result: {}",
        response.verified
    );
    Ok(response.verified)
}
