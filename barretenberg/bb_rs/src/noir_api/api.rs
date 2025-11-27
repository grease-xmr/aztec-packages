use crate::noir_api::inputs::Inputs;
use acir::native_types::WitnessStack;
use acir::FieldElement;
use bn254_blackbox_solver::Bn254BlackBoxSolver;
use log::*;
use nargo::foreign_calls::DefaultForeignCallBuilder;
use nargo::ops::debug::load_workspace_files;
use nargo::ops::{compile_program, execute_program};
use nargo_toml::{get_package_manifest, resolve_workspace_from_toml, PackageSelection};
use noirc_abi::input_parser::InputValue;
use noirc_artifacts::program::ProgramArtifact;
use noirc_driver::{CompileOptions, NOIR_ARTIFACT_VERSION_STRING};
use std::path::Path;
use thiserror::Error;

pub struct CompilationResult {
    pub program: ProgramArtifact,
    pub warnings: Vec<String>,
}

#[derive(Debug, Error)]
pub enum NoirError {
    #[error("Compilation failed with warnings and errors")]
    Compilation {
        warnings: Vec<String>,
        errors: Vec<String>,
    },
    #[error("Execution of contract failed: {0}")]
    Execution(String),
    #[error("Your Nargo workspace is not correctly configured: {0}")]
    Workspace(String),
}

/// Compiles a Noir program located at the given nargo workspace path.
///
/// You only need to call `compile` once for a given contract project. The resulting [`ProgramArtifact`] can be
/// extracted from the returned [`CompilationResult`] and used multiple times for execution with different inputs.
///
/// This is convenient for mobile or resource constrained environments where you want to avoid providing the full Nargo
/// toolchain, but still need a way to generate proofs, and you don't want to or cannot go the WASM route.
///
/// # Arguments
/// - `nargo_path`: The file system path to the root of the nargo workspace containing the Noir project. This folder
/// _must_ contain a `Nargo.toml` manifest file.
/// - `settings`: Compilation settings to customize the compilation process.
pub fn compile(
    nargo_path: impl AsRef<Path>,
    settings: CompileOptions,
) -> Result<CompilationResult, NoirError> {
    let path = nargo_path.as_ref();
    // Load workspace
    let toml_path = get_package_manifest(path).map_err(|e| NoirError::Workspace(e.to_string()))?;

    let workspace = resolve_workspace_from_toml(
        &toml_path,
        PackageSelection::DefaultOrAll,
        Some(NOIR_ARTIFACT_VERSION_STRING.to_owned()),
    )
    .map_err(|e| NoirError::Workspace(e.to_string()))?;
    debug!(
        "Workspace recreated from manifest. {} members found.",
        workspace.members.len()
    );

    let (file_manager, parsed_files) = load_workspace_files(&workspace);
    debug!("File manager created successfully.");
    debug!("{} files parsed.", parsed_files.len());

    let package = workspace
        .into_iter()
        .find(|p| p.is_binary())
        .ok_or_else(|| NoirError::Workspace("No binary package found".to_string()))?;

    debug!(
        "Package {} created from workspace. Entry path: {}",
        package.name,
        package.entry_path.display()
    );

    let (program, warnings) = compile_program(
        &file_manager,
        &parsed_files,
        &workspace,
        package,
        &settings,
        None,
    )
    .map_err(|all| {
        let (warnings, errors): (Vec<_>, Vec<_>) = all.into_iter().partition(|e| e.is_warning());
        let warnings = warnings.into_iter().map(|w| w.to_string()).collect();
        let errors = errors.into_iter().map(|e| e.to_string()).collect();
        NoirError::Compilation { warnings, errors }
    })?;
    debug!("Compilation finished with {} warnings.", warnings.len());

    let warnings = warnings.into_iter().map(|w| w.to_string()).collect();
    let program = ProgramArtifact::from(program);
    Ok(CompilationResult { program, warnings })
}

/// Executes a circuit to calculate its return value
///
/// This function is equivalent to the nargo CLI's `execute` command.
///
/// # Arguments
/// - `program`: The compiled Noir program to execute.
/// - `inputs_map`: A map of input values to provide to the program during execution.
/// - `pedantic_solving`: If true, the solver will perform additional checks during execution.
///
/// # Returns
/// - `WitnessStack<FieldElement>`: The resulting witness stack after execution.
pub fn execute(
    program: &ProgramArtifact,
    inputs: Inputs,
    pedantic_solving: bool,
) -> Result<ExecutionResult, NoirError> {
    // Execute
    let mut foreign_call_executor = DefaultForeignCallBuilder::default()
        .with_output(Vec::<u8>::new())
        .with_mocks(false)
        .build();

    let blackbox_solver = Bn254BlackBoxSolver(pedantic_solving);

    let initial_witness = program
        .abi
        .encode(inputs.as_input_map(), None)
        .map_err(|e| {
            NoirError::Execution(format!(
                "Noir program execution failed when encoding inputs: {e}"
            ))
        })?;

    let witness_stack = execute_program(
        &program.bytecode,
        initial_witness,
        &blackbox_solver,
        &mut foreign_call_executor,
    )
    .map_err(|e| NoirError::Execution(format!("Witness execution failed: {e}")))?;

    let main_witness = &witness_stack
        .peek()
        .ok_or_else(|| NoirError::Execution("No main witness".to_string()))?
        .witness;

    let (_, return_value) = program
        .abi
        .decode(main_witness)
        .map_err(|e| NoirError::Execution(format!("Decoding actual return value failed: {e}")))?;

    Ok(ExecutionResult {
        witness_stack,
        return_value,
    })
}

/// The result of running a circuit with the given inputs.
pub struct ExecutionResult {
    /// The witness stack resulting from the execution.
    pub witness_stack: WitnessStack<FieldElement>,
    /// The return value from the execution, if any.
    pub return_value: Option<InputValue>,
}

#[cfg(test)]
mod tests {
    use crate::noir_api::artifacts::load_artifact;
    use crate::noir_api::inputs::Inputs;
    use noirc_driver::CompileOptions;

    #[test]
    fn compile_noir() {
        let _ = env_logger::try_init();
        let settings = CompileOptions::default();
        let compile_result =
            super::compile("test_vectors/hello_world", settings).expect("Noir compilation failed.");
        assert_eq!(compile_result.warnings.len(), 0);
    }

    #[test]
    fn execute_noir() {
        let artifact =
            load_artifact("test_vectors/hello_world.json").expect("Loading artifact failed");
        let inputs = Inputs::new().add_field("x", 1u64).add_field("y", 2u64);
        let result = super::execute(&artifact, inputs, false).expect("Noir execution failed");
        let bytes = &result
            .witness_stack
            .serialize()
            .expect("serialization failed");
        assert_eq!(bytes.len(), 95, "Witness size should be 95 bytes");
        assert!(result.return_value.is_none());
    }
}
