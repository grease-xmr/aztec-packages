mod api;
pub mod artifacts;
mod inputs;

// exports
pub use api::{compile, execute, CompilationResult, ExecutionResult, NoirError};
pub use inputs::{FieldInput, InputError, Inputs, PointInput, ToInputValue, VecInput};

// re-export
pub use acir::{circuit::Program, bincode_deserialize, bincode_serialize};
pub use noirc_artifacts::program::ProgramArtifact;
pub use noirc_driver::CompileOptions;
