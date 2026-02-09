use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::types::FileKind;
use anyhow::{Context, Result, bail};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct FileMapping {
    pub path: PathBuf,
    pub kind: FileKind,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Manifest {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub files: Vec<FileMapping>,
}

impl Default for Manifest {
    fn default() -> Self {
        Manifest {
            name: "unnamed".to_string(),
            version: "0.0.1".to_string(),
            description: None,
            author: None,
            files: vec![],
        }
    }
}

impl Manifest {
    pub fn with_name(mut self, name: String) -> Self {
        self.name = name;
        self
    }

    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    pub fn with_author(mut self, author: String) -> Self {
        self.author = Some(author);
        self
    }

    pub fn with_version(mut self, version: String) -> Self {
        self.version = version;
        self
    }

    pub fn with_files(mut self, files: Vec<FileMapping>) -> Self {
        self.files = files;
        self
    }

    pub fn add_file(mut self, file: FileMapping) -> Self {
        self.files.push(file);
        self
    }
}

pub fn load_manifest(path: &Path) -> Result<Manifest> {
    if path.is_dir() {
        return load_manifest(&path.join("agentfiles.json"));
    }
    let content = std::fs::read_to_string(path).context("failed to read manifest")?;
    serde_json::from_str(&content).context("failed to parse manifest file")
}

pub fn save_manifest(manifest: &Manifest, path: &Path) -> Result<PathBuf> {
    if path.is_file() {
        bail!("cannot save manifest to a file, provide a directory path.");
    }
    let content = serde_json::to_string_pretty(manifest)?;
    let output_path = path.join("agentfiles.json");
    std::fs::write(&output_path, content).context("failed to write manifest")?;
    Ok(output_path)
}

#[cfg(test)]
mod tests {
    mod load_manifest {
        use super::super::*;
        use tempfile::TempDir;

        #[test]
        fn load_from_directory() -> Result<()> {
            let dir = TempDir::new().unwrap();
            let test_file = dir.path().join("agentfiles.json");

            std::fs::write(
                &test_file,
                r#"{
                "name": "Test Agent",
                "author": "Test Author",
                "version": "0.1.0",
                "files": [
                    {
                        "path": "skill1.py",
                        "kind": "Skill"
                    },
                    {
                        "path": "skill2.py",
                        "kind": "Skill"
                    }
                ]
            }"#,
            )?;

            let manifest = load_manifest(&test_file)?;

            assert_eq!(manifest.author, Some("Test Author".to_string()));
            assert_eq!(manifest.name, "Test Agent");
            assert_eq!(manifest.version, "0.1.0");
            assert_eq!(manifest.files.len(), 2);
            assert_eq!(manifest.files[0].path, PathBuf::from("skill1.py"));
            assert_eq!(manifest.files[0].kind, FileKind::Skill);
            assert_eq!(manifest.files[1].path, PathBuf::from("skill2.py"));
            assert_eq!(manifest.files[1].kind, FileKind::Skill);

            Ok(())
        }
    }

    mod save_manifest {
        use super::super::*;
        use tempfile::TempDir;

        #[test]
        fn save_to_path() -> Result<()> {
            let dir = TempDir::new().unwrap();
            let test_file = dir.path().join("agentfiles.json");
            let manifest = Manifest {
                name: "Test Agent".to_string(),
                version: "0.1.0".to_string(),
                author: Some("Test Author".to_string()),
                files: vec![
                    FileMapping {
                        path: PathBuf::from("skill1.py"),
                        kind: FileKind::Skill,
                    },
                    FileMapping {
                        path: PathBuf::from("skill2.py"),
                        kind: FileKind::Skill,
                    },
                ],
                ..Default::default()
            };

            save_manifest(&manifest, &test_file.parent().unwrap())?;

            let saved_manifest = load_manifest(&test_file)?;

            assert_eq!(saved_manifest.author, Some("Test Author".to_string()));
            assert_eq!(saved_manifest.name, "Test Agent".to_string());
            assert_eq!(saved_manifest.version, "0.1.0".to_string());
            assert_eq!(saved_manifest.files.len(), 2);
            assert_eq!(saved_manifest.files[0].path, PathBuf::from("skill1.py"));
            assert_eq!(saved_manifest.files[0].kind, FileKind::Skill);
            assert_eq!(saved_manifest.files[1].path, PathBuf::from("skill2.py"));
            assert_eq!(saved_manifest.files[1].kind, FileKind::Skill);

            Ok(())
        }

        #[test]
        fn save_to_file_error() -> Result<()> {
            let dir = TempDir::new().unwrap();
            let test_file = dir.path().join("agentfiles.json");
            let manifest = Manifest {
                name: "Test Agent".to_string(),
                version: "0.1.0".to_string(),
                author: Some("Test Author".to_string()),
                files: vec![
                    FileMapping {
                        path: PathBuf::from("skill1.py"),
                        kind: FileKind::Skill,
                    },
                    FileMapping {
                        path: PathBuf::from("skill2.py"),
                        kind: FileKind::Skill,
                    },
                ],
                ..Default::default()
            };

            let result = save_manifest(&manifest, &test_file);

            assert!(result.is_err());

            Ok(())
        }
    }
}
