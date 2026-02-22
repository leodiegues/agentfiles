use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use log::debug;
use serde::{Deserialize, Serialize};

use crate::git;
use crate::types::{FileKind, FileStrategy};

/// A single discovered agent file used by the scanner and installer.
///
/// Not serialized into the manifest — this is an in-memory representation
/// of a file to be installed.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct FileMapping {
    /// Path to the source file or directory, relative to the source root.
    pub path: PathBuf,

    /// What kind of agent file this is.
    pub kind: FileKind,

    /// How to place the file at the target. Defaults to Copy.
    pub strategy: FileStrategy,
}

/// A dependency source -- either a simple URL/path string or a detailed spec.
///
/// Simple form: `"github.com/org/repo"` or `"github.com/org/repo@v1.0"`
/// Detailed form: `{ "source": "...", "ref": "v1.0", "pick": [...], ... }`
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(untagged)]
pub enum Dependency {
    Simple(String),
    Detailed(DependencySpec),
}

impl Dependency {
    /// The source URL or local path for this dependency.
    pub fn source(&self) -> &str {
        match self {
            Dependency::Simple(s) => s,
            Dependency::Detailed(d) => &d.source,
        }
    }

    /// Returns the detailed spec, if this is a Detailed dependency.
    pub fn spec(&self) -> Option<&DependencySpec> {
        match self {
            Dependency::Simple(_) => None,
            Dependency::Detailed(d) => Some(d),
        }
    }

    /// Explicit git ref override, if any. Note that `@ref` in Simple strings
    /// is handled by git::parse_remote, not here.
    pub fn git_ref(&self) -> Option<&str> {
        self.spec().and_then(|d| d.git_ref.as_deref())
    }

    /// Cherry-pick list, if any.
    pub fn pick(&self) -> Option<&[String]> {
        self.spec().and_then(|d| d.pick.as_deref())
    }

    /// Per-dependency strategy override, if any.
    pub fn strategy(&self) -> Option<FileStrategy> {
        self.spec().and_then(|d| d.strategy)
    }

    /// Custom path-to-kind mappings. When set, replaces the default
    /// `skills/`, `commands/`, `agents/` scanning convention.
    pub fn paths(&self) -> Option<&[PathMapping]> {
        self.spec().and_then(|d| d.paths.as_deref())
    }
}

/// Detailed dependency specification with optional configuration.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct DependencySpec {
    /// Source URL or local path.
    pub source: String,

    /// Git ref (branch, tag, or commit) to check out.
    #[serde(rename = "ref", skip_serializing_if = "Option::is_none")]
    pub git_ref: Option<String>,

    /// Cherry-pick specific items by name. Supports kind prefix:
    /// `"skills/review"`, `"commands/deploy"`, or plain `"review"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pick: Option<Vec<String>>,

    /// Override the installation strategy for all files from this dependency.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strategy: Option<FileStrategy>,

    /// Custom directory/file-to-kind mappings. Replaces the default convention
    /// when specified.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paths: Option<Vec<PathMapping>>,
}

/// Maps a custom path in a source repository to a file kind.
///
/// If the path resolves to a directory, it is scanned using the standard
/// convention for that kind (skills = subdirs with SKILL.md, commands/agents
/// = .md files). If it resolves to a file, that file is installed directly.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PathMapping {
    /// Relative path within the source repository.
    pub path: String,

    /// What kind of agent file this path contains.
    pub kind: FileKind,
}

/// The agentfiles.json project manifest.
///
/// Lists dependencies (remote or local sources) that provide agent files.
/// Similar to package.json -- lives in the consumer's project.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Manifest {
    pub name: String,

    #[serde(default = "default_version")]
    pub version: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependencies: Vec<Dependency>,
}

fn default_version() -> String {
    "0.0.1".to_string()
}

impl Default for Manifest {
    fn default() -> Self {
        Manifest {
            name: "unnamed".to_string(),
            version: default_version(),
            description: None,
            author: None,
            repository: None,
            dependencies: vec![],
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

    pub fn with_dependencies(mut self, dependencies: Vec<Dependency>) -> Self {
        self.dependencies = dependencies;
        self
    }

    /// Add a dependency if one with the same source doesn't already exist.
    /// Returns true if the dependency was added, false if it was already present.
    pub fn add_dependency(&mut self, dep: Dependency) -> bool {
        let source = dep.source().to_string();
        if self.has_dependency(&source) {
            debug!("Dependency already exists: {}", source);
            return false;
        }
        debug!("Adding dependency: {}", source);
        self.dependencies.push(dep);
        true
    }

    /// Check whether a dependency with the given source already exists.
    ///
    /// Compares using normalized URLs so that `github.com/org/repo` and
    /// `https://github.com/org/repo.git` are treated as the same source.
    pub fn has_dependency(&self, source: &str) -> bool {
        let normalized = git::normalize_source(source);
        self.dependencies
            .iter()
            .any(|d| git::normalize_source(d.source()) == normalized)
    }

    /// Remove a dependency by source. Returns true if a dependency was removed.
    ///
    /// Uses normalized URL comparison, same as `has_dependency`.
    pub fn remove_dependency(&mut self, source: &str) -> bool {
        debug!("Removing dependency: {}", source);
        let normalized = git::normalize_source(source);
        let before = self.dependencies.len();
        self.dependencies
            .retain(|d| git::normalize_source(d.source()) != normalized);
        self.dependencies.len() < before
    }
}

/// Load a manifest from a file path or directory.
///
/// If `path` is a directory, looks for `agentfiles.json` inside it.
pub fn load_manifest(path: &Path) -> Result<Manifest> {
    debug!("Loading manifest from {}", path.display());
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
    debug!("Saving manifest to {}", path.display());
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
    mod dependency_serialization {
        use super::super::*;

        #[test]
        fn simple_dependency_roundtrip() -> Result<()> {
            let dep = Dependency::Simple("github.com/org/repo".to_string());
            let json = serde_json::to_string(&dep)?;
            assert_eq!(json, r#""github.com/org/repo""#);

            let parsed: Dependency = serde_json::from_str(&json)?;
            assert_eq!(parsed, dep);
            assert_eq!(parsed.source(), "github.com/org/repo");
            assert_eq!(parsed.git_ref(), None);
            assert_eq!(parsed.pick(), None);
            assert_eq!(parsed.strategy(), None);
            assert_eq!(parsed.paths(), None);
            Ok(())
        }

        #[test]
        fn detailed_dependency_roundtrip() -> Result<()> {
            let dep = Dependency::Detailed(DependencySpec {
                source: "github.com/org/repo".to_string(),
                git_ref: Some("v2.0".to_string()),
                pick: Some(vec![
                    "skills/review".to_string(),
                    "commands/deploy".to_string(),
                ]),
                strategy: Some(FileStrategy::Link),
                paths: Some(vec![PathMapping {
                    path: "prompts".to_string(),
                    kind: FileKind::Skill,
                }]),
            });

            let json = serde_json::to_string_pretty(&dep)?;
            let parsed: Dependency = serde_json::from_str(&json)?;
            assert_eq!(parsed, dep);

            assert_eq!(parsed.source(), "github.com/org/repo");
            assert_eq!(parsed.git_ref(), Some("v2.0"));
            assert_eq!(parsed.pick().unwrap().len(), 2);
            assert_eq!(parsed.strategy(), Some(FileStrategy::Link));
            assert_eq!(parsed.paths().unwrap().len(), 1);
            Ok(())
        }

        #[test]
        fn detailed_dependency_minimal() -> Result<()> {
            let json = r#"{"source": "github.com/org/repo"}"#;
            let parsed: Dependency = serde_json::from_str(json)?;
            assert_eq!(parsed.source(), "github.com/org/repo");
            assert_eq!(parsed.git_ref(), None);
            assert_eq!(parsed.pick(), None);
            assert_eq!(parsed.strategy(), None);
            assert_eq!(parsed.paths(), None);
            Ok(())
        }

        #[test]
        fn ref_field_uses_serde_rename() -> Result<()> {
            let json = r#"{"source": "github.com/org/repo", "ref": "main"}"#;
            let parsed: Dependency = serde_json::from_str(json)?;
            assert_eq!(parsed.git_ref(), Some("main"));

            // Serializes back as "ref", not "git_ref"
            let serialized = serde_json::to_string(&parsed)?;
            assert!(serialized.contains(r#""ref":"main""#));
            assert!(!serialized.contains("git_ref"));
            Ok(())
        }
    }

    mod manifest_serialization {
        use super::super::*;
        use tempfile::TempDir;

        #[test]
        fn save_and_roundtrip() -> Result<()> {
            let dir = TempDir::new()?;
            let manifest = Manifest {
                name: "my-project".to_string(),
                version: "0.1.0".to_string(),
                author: Some("Test Author".to_string()),
                repository: Some("https://github.com/org/repo".to_string()),
                dependencies: vec![
                    Dependency::Simple("github.com/anthropics/skills".to_string()),
                    Dependency::Detailed(DependencySpec {
                        source: "github.com/mitsuhiko/agent-stuff".to_string(),
                        git_ref: Some("main".to_string()),
                        pick: Some(vec!["skills/commit".to_string()]),
                        strategy: None,
                        paths: None,
                    }),
                ],
                ..Default::default()
            };

            save_manifest(&manifest, dir.path())?;
            let loaded = load_manifest(dir.path())?;

            assert_eq!(loaded.name, "my-project");
            assert_eq!(loaded.version, "0.1.0");
            assert_eq!(loaded.author, Some("Test Author".to_string()));
            assert_eq!(loaded.dependencies.len(), 2);
            assert_eq!(
                loaded.dependencies[0].source(),
                "github.com/anthropics/skills"
            );
            assert_eq!(
                loaded.dependencies[1].source(),
                "github.com/mitsuhiko/agent-stuff"
            );
            assert_eq!(loaded.dependencies[1].git_ref(), Some("main"));
            Ok(())
        }

        #[test]
        fn empty_dependencies_not_serialized() -> Result<()> {
            let dir = TempDir::new()?;
            let manifest = Manifest::default();
            save_manifest(&manifest, dir.path())?;

            let content = std::fs::read_to_string(dir.path().join("agentfiles.json"))?;
            assert!(!content.contains("dependencies"));
            Ok(())
        }

        #[test]
        fn load_from_directory_path() -> Result<()> {
            let dir = TempDir::new()?;
            std::fs::write(
                dir.path().join("agentfiles.json"),
                r#"{"name": "test", "version": "0.1.0"}"#,
            )?;
            let manifest = load_manifest(dir.path())?;
            assert_eq!(manifest.name, "test");
            assert!(manifest.dependencies.is_empty());
            Ok(())
        }

        #[test]
        fn save_to_file_error() -> Result<()> {
            let dir = TempDir::new()?;
            let file = dir.path().join("agentfiles.json");
            std::fs::write(&file, "{}")?;
            let result = save_manifest(&Manifest::default(), &file);
            assert!(result.is_err());
            Ok(())
        }
    }

    mod manifest_helpers {
        use super::super::*;

        #[test]
        fn add_dependency_deduplicates() {
            let mut manifest = Manifest::default();
            let dep = Dependency::Simple("github.com/org/repo".to_string());

            assert!(manifest.add_dependency(dep.clone()));
            assert!(!manifest.add_dependency(dep));
            assert_eq!(manifest.dependencies.len(), 1);
        }

        #[test]
        fn has_dependency_checks_source() {
            let mut manifest = Manifest::default();
            manifest
                .dependencies
                .push(Dependency::Simple("github.com/org/repo".to_string()));

            assert!(manifest.has_dependency("github.com/org/repo"));
            assert!(!manifest.has_dependency("github.com/other/repo"));
        }

        #[test]
        fn builder_methods() {
            let manifest = Manifest::default()
                .with_name("test".to_string())
                .with_version("1.0.0".to_string())
                .with_description("A test project".to_string())
                .with_author("Author".to_string())
                .with_repository("https://github.com/test".to_string())
                .with_dependencies(vec![Dependency::Simple("dep1".to_string())]);

            assert_eq!(manifest.name, "test");
            assert_eq!(manifest.version, "1.0.0");
            assert_eq!(manifest.description, Some("A test project".to_string()));
            assert_eq!(manifest.author, Some("Author".to_string()));
            assert_eq!(manifest.dependencies.len(), 1);
        }
    }

    mod path_mapping {
        use super::super::*;

        #[test]
        fn path_mapping_roundtrip() -> Result<()> {
            let mapping = PathMapping {
                path: "prompts".to_string(),
                kind: FileKind::Skill,
            };
            let json = serde_json::to_string(&mapping)?;
            let parsed: PathMapping = serde_json::from_str(&json)?;
            assert_eq!(parsed, mapping);
            Ok(())
        }

        #[test]
        fn dependency_with_custom_paths() -> Result<()> {
            let json = r#"{
                "source": "github.com/some/repo",
                "paths": [
                    {"path": "prompts", "kind": "skill"},
                    {"path": "macros", "kind": "command"}
                ]
            }"#;
            let dep: Dependency = serde_json::from_str(json)?;
            let paths = dep.paths().unwrap();
            assert_eq!(paths.len(), 2);
            assert_eq!(paths[0].path, "prompts");
            assert_eq!(paths[0].kind, FileKind::Skill);
            assert_eq!(paths[1].path, "macros");
            assert_eq!(paths[1].kind, FileKind::Command);
            Ok(())
        }
    }
}
