mod api;
pub mod artifacts;

// exports
pub use api::{compile, NoirError};

// re-export
pub use acir::circuit::Program;
pub use noirc_artifacts::program::ProgramArtifact;
