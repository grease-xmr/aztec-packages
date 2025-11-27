mod barretenberg_api;
pub mod noir_api;

pub use barretenberg_api::bbapi::{CircuitComputeVk, CircuitProve, CircuitVerify};

pub mod circuits {
    pub use crate::barretenberg_api::acir::{
        acir_get_slow_low_memory, acir_set_slow_low_memory, get_circuit_sizes, CircuitSizes,
    };
}

pub mod ultra_honk {
    pub use crate::barretenberg_api::bbapi::{
        get_ultra_honk_verification_key as get_vk, prove_ultra_honk as prove,
        verify_ultra_honk as verify,
    };
}

pub mod ultra_honk_keccak {
    pub use crate::barretenberg_api::bbapi::{
        get_ultra_honk_keccak_verification_key as get_vk, prove_ultra_keccak_honk as prove,
        verify_ultra_keccak_honk as verify,
    };
}

pub mod ultra_honk_keccak_zk {
    pub use crate::barretenberg_api::bbapi::{
        get_ultra_honk_keccak_zk_verification_key as get_vk, prove_ultra_keccak_zk_honk as prove,
        verify_ultra_keccak_zk_honk as verify,
    };
}

pub use barretenberg_api::models;
