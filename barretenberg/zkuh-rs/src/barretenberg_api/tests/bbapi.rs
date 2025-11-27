use crate::barretenberg_api::bbapi::{BbApiError, CircuitComputeVkResponse, CircuitProveResponse};
use crate::noir_api::artifacts::load_artifact;
use crate::noir_api::artifacts::load_binary;
use crate::ultra_honk;
use acir::bincode_serialize;

#[test]
fn ultra_honk_prove() {
    let _ = env_logger::try_init();
    let program = load_artifact("test_vectors/hello_world.json").expect("Load artifact");
    let constraints = bincode_serialize(&program.bytecode).expect("Bincode serialization");
    let witness = load_binary("test_vectors/hello_world_witness.gz").expect("witness load failed");
    let proof = ultra_honk::prove(&constraints, &witness, &[]).expect("proving failed");
    assert_eq!(
        proof.public_inputs.len(),
        1,
        "There should be 1 public inputs"
    );
    let y = proof.public_inputs[0].as_bigint().to_string();
    assert_eq!(y, "2");
    assert_eq!(
        proof.proof.len(),
        508,
        "Proof length should be 508 elements"
    );
    // Save the proof for verification test
    // proof.save("test_vectors/hello_world_proof.gz").expect("Proof save failed");
}

#[test]
fn ultra_honk_verify() {
    let _ = env_logger::try_init();
    let proof =
        CircuitProveResponse::load("test_vectors/hello_world_proof.gz").expect("Proof load failed");
    assert_eq!(
        proof.proof.len(),
        508,
        "Proof length should be 508 elements"
    );
    let result = ultra_honk::verify(proof).expect("verify failed");
    assert!(result, "Proof should be valid");
}

#[test]
fn ultra_honk_verify_with_empty_proof() {
    let proof = CircuitProveResponse {
        proof: vec![],
        public_inputs: vec![],
        vk: CircuitComputeVkResponse {
            bytes: vec![],
            fields: vec![],
            hash: vec![],
        },
    };
    let result = ultra_honk::verify(proof);
    assert!(matches!(&result, Err(BbApiError::ApiError(msg)) if msg == "Proof cannot be empty"));
}
