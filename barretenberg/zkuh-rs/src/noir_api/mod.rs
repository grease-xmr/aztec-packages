mod api;
pub mod artifacts;
mod inputs;

// exports
pub use api::{compile, execute, CompilationResult, ExecutionResult, NoirError};
pub use inputs::{FieldInput, InputError, Inputs, PointInput, ToInputValue, VecInput, PublicInputError};

// re-export
pub use acir::{bincode_deserialize, bincode_serialize, circuit::Program};
pub use noirc_artifacts::program::ProgramArtifact;
pub use noirc_driver::CompileOptions;
