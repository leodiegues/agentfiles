use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::types::{FileKind, FileStrategy};
use anyhow::{Context, Result, bail};

/// A single file mapping in the manifest.
///
/// Declares a source file path, its kind (Skill/Agent/Command), and an optional
/// installation strategy (Copy or Link). The CLI routes this file to the correct
/// provider directories based on the compatibility matrix.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct FileMapping {
    /// Path to the source file, relative to the manifest location.
    pub path: PathBuf,

    /// What kind of agent file this is.
    pub kind: FileKind,

    /// How to place the file at the target. Defaults to Copy if omitted.
    #[serde(default, skip_serializing_if = "is_default_strategy")]
    pub strategy: FileStrategy,
}

fn is_default_strategy(s: &FileStrategy) -> bool {
    *s == FileStrategy::Copy
}

/// The agentfiles.json manifest.
///
/// Declares a package of agent files with metadata and a list of file mappings.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Manifest {
    pub name: String,
    pub version: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,

    pub files: Vec<FileMapping>,
}

impl Default for Manifest {
    fn default() -> Self {
        Manifest {
            name: "unnamed".to_string(),
            version: "0.0.1".to_string(),
            description: None,
            author: None,
            repository: None,
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

    pub fn with_repository(mut self, repository: String) -> Self {
        self.repository = Some(repository);
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

/// Load a manifest from a file path or directory.
///
/// If `path` is a directory, looks for `agentfiles.json` inside it.
pub fn load_manifest(path: &Path) -> Result<Manifest> {
    if path.is_dir() {
        return load_manifest(&path.join("agentfiles.json"));
    }
    let content = std::fs::read_to_string(path).context("failed to read manifest")?;
    serde_json::from_str(&content).context("failed to parse manifest file")
}

/// Save a manifest to a directory as `agentfiles.json`.
///
/// Returns the full path of the written file.
/// Errors if `path` points to an existing file.
pub fn save_manifest(manifest: &Manifest, path: &Path) -> Result<PathBuf> {
    if path.is_file() {
        bail!("cannot save manifest to a file, provide a directory path.");
    }
    let content = serde_json::to_string_pretty(manifest)? + "\n";
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
                        "path": "skills/review/SKILL.md",
                        "kind": "Skill"
                    },
                    {
                        "path": "commands/deploy.md",
                        "kind": "Command",
                        "strategy": "Link"
                    }
                ]
            }"#,
            )?;

            let manifest = load_manifest(&test_file)?;

            assert_eq!(manifest.author, Some("Test Author".to_string()));
            assert_eq!(manifest.name, "Test Agent");
            assert_eq!(manifest.version, "0.1.0");
            assert_eq!(manifest.files.len(), 2);

            assert_eq!(
                manifest.files[0].path,
                PathBuf::from("skills/review/SKILL.md")
            );
            assert_eq!(manifest.files[0].kind, FileKind::Skill);
            assert_eq!(manifest.files[0].strategy, FileStrategy::Copy); // default

            assert_eq!(manifest.files[1].path, PathBuf::from("commands/deploy.md"));
            assert_eq!(manifest.files[1].kind, FileKind::Command);
            assert_eq!(manifest.files[1].strategy, FileStrategy::Link);

            Ok(())
        }

        #[test]
        fn load_from_directory_path() -> Result<()> {
            let dir = TempDir::new().unwrap();
            let test_file = dir.path().join("agentfiles.json");

            std::fs::write(
                &test_file,
                r#"{
                "name": "Test",
                "version": "0.1.0",
                "files": []
            }"#,
            )?;

            // Pass the directory, not the file
            let manifest = load_manifest(dir.path())?;
            assert_eq!(manifest.name, "Test");

            Ok(())
        }
    }

    mod save_manifest {
        use super::super::*;
        use tempfile::TempDir;

        #[test]
        fn save_and_roundtrip() -> Result<()> {
            let dir = TempDir::new().unwrap();
            let manifest = Manifest {
                name: "Test Agent".to_string(),
                version: "0.1.0".to_string(),
                author: Some("Test Author".to_string()),
                repository: Some("https://github.com/org/repo".to_string()),
                files: vec![
                    FileMapping {
                        path: PathBuf::from("skills/review/SKILL.md"),
                        kind: FileKind::Skill,
                        strategy: FileStrategy::Copy,
                    },
                    FileMapping {
                        path: PathBuf::from("commands/deploy.md"),
                        kind: FileKind::Command,
                        strategy: FileStrategy::Link,
                    },
                ],
                ..Default::default()
            };

            save_manifest(&manifest, dir.path())?;

            let loaded = load_manifest(dir.path())?;

            assert_eq!(loaded.author, Some("Test Author".to_string()));
            assert_eq!(loaded.name, "Test Agent");
            assert_eq!(loaded.version, "0.1.0");
            assert_eq!(
                loaded.repository,
                Some("https://github.com/org/repo".to_string())
            );
            assert_eq!(loaded.files.len(), 2);
            assert_eq!(loaded.files[0].kind, FileKind::Skill);
            assert_eq!(loaded.files[0].strategy, FileStrategy::Copy);
            assert_eq!(loaded.files[1].kind, FileKind::Command);
            assert_eq!(loaded.files[1].strategy, FileStrategy::Link);

            Ok(())
        }

        #[test]
        fn save_to_file_error() -> Result<()> {
            let dir = TempDir::new().unwrap();
            let test_file = dir.path().join("agentfiles.json");

            // Create the file first so is_file() returns true
            std::fs::write(&test_file, "{}")?;

            let manifest = Manifest::default();
            let result = save_manifest(&manifest, &test_file);
            assert!(result.is_err());

            Ok(())
        }

        #[test]
        fn default_strategy_not_serialized() -> Result<()> {
            let dir = TempDir::new().unwrap();
            let manifest = Manifest {
                name: "Test".to_string(),
                version: "0.1.0".to_string(),
                files: vec![FileMapping {
                    path: PathBuf::from("skills/test/SKILL.md"),
                    kind: FileKind::Skill,
                    strategy: FileStrategy::Copy, // default, should be omitted from JSON
                }],
                ..Default::default()
            };

            save_manifest(&manifest, dir.path())?;

            let content = std::fs::read_to_string(dir.path().join("agentfiles.json"))?;
            // The "strategy" key should NOT appear for Copy (the default)
            assert!(!content.contains("strategy"));

            Ok(())
        }
    }
}
