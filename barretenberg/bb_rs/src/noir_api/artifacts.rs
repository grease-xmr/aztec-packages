use flate2::read::GzDecoder;
use noirc_artifacts::program::ProgramArtifact;
use std::fs::File;
use std::io::Read;
use std::path::Path;

pub fn load_artifact(path: impl AsRef<Path>) -> Result<ProgramArtifact, std::io::Error> {
    let json = std::fs::read_to_string(path)?;
    serde_json::from_str::<ProgramArtifact>(&json).map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Failed to deserialize ProgramArtifact: {}", e),
        )
    })
}

pub fn load_witness(path: impl AsRef<Path>) -> Result<Vec<u8>, std::io::Error> {
    let path = path.as_ref();
    let is_compressed = path.extension().map(|ext| ext == "gz").unwrap_or(false);
    if is_compressed {
        load_compressed_witness(path)
    } else {
        load_uncompressed_witness(path)
    }
}

fn load_uncompressed_witness(path: &Path) -> Result<Vec<u8>, std::io::Error> {
    std::fs::read(path)
}

fn load_compressed_witness(path: &Path) -> Result<Vec<u8>, std::io::Error> {
    let file = File::open(path)?;
    let mut decoder = GzDecoder::new(file);
    let mut decompressed_data = Vec::new();
    decoder.read_to_end(&mut decompressed_data)?;
    Ok(decompressed_data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_artifact() {
        let loaded_artifact = load_artifact("test_vectors/hello_world.json").unwrap();
        assert_eq!(
            loaded_artifact.noir_version,
            "1.0.0-beta.15+83245db91dcf63420ef4bcbbd85b98f397fee663"
        );
        assert_eq!(loaded_artifact.hash, 9763453774353198784);
        assert_eq!(loaded_artifact.abi.parameters[0].name, "x");
        assert!(!loaded_artifact.abi.parameters[0].is_public());
        assert_eq!(loaded_artifact.abi.parameters[1].name, "y");
        assert!(loaded_artifact.abi.parameters[1].is_public());
        assert_eq!(loaded_artifact.bytecode.functions.len(), 1);
    }
}
