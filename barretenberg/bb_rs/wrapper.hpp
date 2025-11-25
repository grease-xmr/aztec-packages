#include <barretenberg/crypto/pedersen_commitment/c_bind.hpp>
#include <barretenberg/crypto/pedersen_hash/c_bind.hpp>
#include <barretenberg/crypto/poseidon2/c_bind.hpp>
#include <barretenberg/crypto/blake2s/c_bind.hpp>
#include <barretenberg/crypto/aes128/c_bind.hpp>
#include <barretenberg/crypto/schnorr/c_bind.hpp>
#include <barretenberg/crypto/ecdsa/c_bind.h>
#include <barretenberg/ecc/curves/secp256k1/c_bind.hpp>
#include <barretenberg/srs/c_bind.hpp>
#include <barretenberg/common/c_bind.hpp>
#include <barretenberg/dsl/acir_proofs/c_bind.hpp>
#include <barretenberg/bbapi/c_bind.hpp>

// Grumpkin function declarations (no header file exists)
extern "C" {
    void ecc_grumpkin__mul(uint8_t const* point_buf, uint8_t const* scalar_buf, uint8_t* result);
    void ecc_grumpkin__add(uint8_t const* point_a_buf, uint8_t const* point_b_buf, uint8_t* result);
    void ecc_grumpkin__batch_mul(uint8_t const* point_buf, uint8_t const* scalar_buf, uint32_t num_points, uint8_t* result);
    void ecc_grumpkin__get_random_scalar_mod_circuit_modulus(uint8_t* result);
    void ecc_grumpkin__reduce512_buffer_mod_circuit_modulus(uint8_t* input, uint8_t* result);

    // BN254 function declarations (no header file exists)
    void bn254_fr_sqrt(uint8_t const* input, uint8_t* result);
}
