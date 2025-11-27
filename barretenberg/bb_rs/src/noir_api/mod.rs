mod api;
pub mod artifacts;
mod inputs;

// exports
pub use api::{compile, execute, CompilationResult, ExecutionResult, NoirError};
pub use inputs::Inputs;
// re-export
pub use acir::circuit::Program;
pub use noirc_artifacts::program::ProgramArtifact;
