use num_bigint::BigUint;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct Uint256(#[serde(with = "serde_bytes")] [u8; 32]);

impl Uint256 {
    pub fn as_bigint(&self) -> BigUint {
        BigUint::from_bytes_be(&self.0)
    }
}

impl From<[u8; 32]> for Uint256 {
    fn from(bytes: [u8; 32]) -> Self {
        Uint256(bytes)
    }
}

impl From<u8> for Uint256 {
    fn from(value: u8) -> Self {
        let mut bytes = [0u8; 32];
        bytes[31] = value;
        Uint256(bytes)
    }
}

pub fn bytes_to_uint256(bytes: &[u8]) -> Result<Vec<Uint256>, &'static str> {
    if bytes.len() % 32 != 0 {
        return Err("Input byte slice length must be a multiple of 32");
    }
    let (chunks, []) = bytes.as_chunks::<32>() else {
        return Err("Failed to split byte slice into 32-byte chunks");
    };
    Ok(chunks.iter().map(|arr| Uint256::from(*arr)).collect())
}

pub fn uint256_to_bytes(uints: &[Uint256]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(uints.len() * 32);
    for uint in uints {
        bytes.extend_from_slice(&uint.0);
    }
    bytes
}
