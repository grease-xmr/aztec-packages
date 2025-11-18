use crate::noir_api::artifacts::load_witness;
use crate::noir_api::{artifacts::load_artifact, Program};
use crate::ultra_honk;
use acir::bincode_serialize;

#[test]
fn prove_ultra_honk() {
    let _ = env_logger::try_init();
    let program = load_artifact("test_vectors/hello_world.json").expect("Load artifact");
    let constraints = bincode_serialize(&program.bytecode).expect("Bincode serialization");
    let witness = load_witness("test_vectors/hello_world_witness.gz").expect("witness load failed");
    let proof = ultra_honk::prove(&constraints, &witness, &[]).expect("proving failed");
    assert_eq!(proof.len(), 16256, "Proof length should be 16256 bytes");
}
