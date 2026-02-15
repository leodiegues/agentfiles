use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use crate::manifest::Manifest;
use crate::types::{AgentProvider, FileScope, FileStrategy};

/// Result of installing a single file to a single provider.
#[derive(Debug)]
pub struct InstallResult {
    pub provider: AgentProvider,
    pub source: String,
    pub target: String,
    pub strategy: FileStrategy,
    pub kind: String,
}

/// Install all files from a manifest to the specified providers.
///
/// For each file in the manifest, iterates over `providers` and installs the file
/// to every provider that supports the file's kind. The `manifest_dir` is the
/// directory containing the manifest (used to resolve relative source paths).
///
/// Returns a list of `InstallResult` entries describing what was installed.
pub fn install(
    manifest: &Manifest,
    providers: &[AgentProvider],
    scope: &FileScope,
    project_root: &Path,
    manifest_dir: &Path,
) -> Result<Vec<InstallResult>> {
    let mut results = Vec::new();

    for file in &manifest.files {
        let source_path = manifest_dir.join(&file.path);

        if !source_path.exists() {
            anyhow::bail!(
                "source file not found: {} (resolved to {})",
                file.path.display(),
                source_path.display()
            );
        }

        for provider in providers {
            if !provider.supports_kind(&file.kind) {
                continue;
            }

            let target_dir = provider.get_target_dir(scope, &file.kind, project_root)?;
            let target_path = resolve_target_path(&file.path, &target_dir)?;

            if let Some(parent) = target_path.parent() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("failed to create directory: {}", parent.display()))?;
            }

            // Place the file
            match file.strategy {
                FileStrategy::Copy => {
                    if source_path.is_dir() {
                        copy_dir_recursive(&source_path, &target_path)?;
                    } else {
                        fs::copy(&source_path, &target_path).with_context(|| {
                            format!(
                                "failed to copy {} -> {}",
                                source_path.display(),
                                target_path.display()
                            )
                        })?;
                    }
                }
                FileStrategy::Link => {
                    if target_path.exists() || target_path.is_symlink() {
                        if target_path.is_dir() && !target_path.is_symlink() {
                            fs::remove_dir_all(&target_path)?;
                        } else {
                            fs::remove_file(&target_path)?;
                        }
                    }

                    let abs_source = source_path.canonicalize().with_context(|| {
                        format!("failed to resolve absolute path: {}", source_path.display())
                    })?;

                    #[cfg(unix)]
                    std::os::unix::fs::symlink(&abs_source, &target_path).with_context(|| {
                        format!(
                            "failed to symlink {} -> {}",
                            abs_source.display(),
                            target_path.display()
                        )
                    })?;

                    #[cfg(windows)]
                    {
                        if abs_source.is_dir() {
                            std::os::windows::fs::symlink_dir(&abs_source, &target_path)
                                .with_context(|| {
                                    format!(
                                        "failed to symlink {} -> {}",
                                        abs_source.display(),
                                        target_path.display()
                                    )
                                })?;
                        } else {
                            std::os::windows::fs::symlink_file(&abs_source, &target_path)
                                .with_context(|| {
                                    format!(
                                        "failed to symlink {} -> {}",
                                        abs_source.display(),
                                        target_path.display()
                                    )
                                })?;
                        }
                    }
                }
            }

            results.push(InstallResult {
                provider: *provider,
                source: file.path.display().to_string(),
                target: target_path.display().to_string(),
                strategy: file.strategy,
                kind: file.kind.to_string(),
            });
        }
    }

    Ok(results)
}

/// Resolve where the file should land inside the target directory.
///
/// For skills (paths containing SKILL.md), we place the entire skill directory
/// (e.g., `review/SKILL.md` -> `<target_dir>/review/SKILL.md`).
///
/// For commands/agents (single .md files), we place the file directly
/// (e.g., `deploy.md` -> `<target_dir>/deploy.md`).
fn resolve_target_path(relative_path: &Path, target_dir: &Path) -> Result<std::path::PathBuf> {
    let file_name = relative_path
        .file_name()
        .context("file path has no filename")?;

    if file_name == "SKILL.md" {
        let parent_name = relative_path
            .parent()
            .and_then(|p| p.file_name())
            .context("SKILL.md must be inside a named directory")?;
        Ok(target_dir.join(parent_name).join("SKILL.md"))
    } else {
        Ok(target_dir.join(file_name))
    }
}

/// Recursively copy a directory, skipping symlinks to avoid infinite loops.
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if file_type.is_symlink() {
            // Skip symlinks to prevent infinite recursion from directory loops
            continue;
        } else if file_type.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::FileMapping;
    use crate::types::{FileKind, FileStrategy};
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn make_manifest(files: Vec<FileMapping>) -> Manifest {
        Manifest {
            name: "test".to_string(),
            version: "0.1.0".to_string(),
            files,
            ..Default::default()
        }
    }

    #[test]
    fn install_skill_to_claude_code() -> Result<()> {
        let src_dir = TempDir::new()?;
        let dst_dir = TempDir::new()?;

        // Create source skill
        let skill_dir = src_dir.path().join("skills").join("review");
        fs::create_dir_all(&skill_dir)?;
        fs::write(skill_dir.join("SKILL.md"), "# Review skill")?;

        let manifest = make_manifest(vec![FileMapping {
            path: PathBuf::from("skills/review/SKILL.md"),
            kind: FileKind::Skill,
            strategy: FileStrategy::Copy,
        }]);

        let results = install(
            &manifest,
            &[AgentProvider::ClaudeCode],
            &FileScope::Project,
            dst_dir.path(),
            src_dir.path(),
        )?;

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].provider, AgentProvider::ClaudeCode);

        // Verify the file was copied
        let target = dst_dir.path().join(".claude/skills/review/SKILL.md");
        assert!(target.exists());
        assert_eq!(fs::read_to_string(&target)?, "# Review skill");

        Ok(())
    }

    #[test]
    fn install_command_skips_codex() -> Result<()> {
        let src_dir = TempDir::new()?;
        let dst_dir = TempDir::new()?;

        // Create source command
        let cmd_dir = src_dir.path().join("commands");
        fs::create_dir_all(&cmd_dir)?;
        fs::write(cmd_dir.join("deploy.md"), "# Deploy")?;

        let manifest = make_manifest(vec![FileMapping {
            path: PathBuf::from("commands/deploy.md"),
            kind: FileKind::Command,
            strategy: FileStrategy::Copy,
        }]);

        let results = install(
            &manifest,
            &[AgentProvider::Codex, AgentProvider::ClaudeCode],
            &FileScope::Project,
            dst_dir.path(),
            src_dir.path(),
        )?;

        // Should only install to ClaudeCode, not Codex
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].provider, AgentProvider::ClaudeCode);

        Ok(())
    }

    #[test]
    fn install_to_multiple_providers() -> Result<()> {
        let src_dir = TempDir::new()?;
        let dst_dir = TempDir::new()?;

        // Create source skill
        let skill_dir = src_dir.path().join("skills").join("review");
        fs::create_dir_all(&skill_dir)?;
        fs::write(skill_dir.join("SKILL.md"), "# Review")?;

        let manifest = make_manifest(vec![FileMapping {
            path: PathBuf::from("skills/review/SKILL.md"),
            kind: FileKind::Skill,
            strategy: FileStrategy::Copy,
        }]);

        let results = install(
            &manifest,
            AgentProvider::ALL,
            &FileScope::Project,
            dst_dir.path(),
            src_dir.path(),
        )?;

        // All 4 providers support skills
        assert_eq!(results.len(), 4);

        // Verify all targets exist
        assert!(
            dst_dir
                .path()
                .join(".claude/skills/review/SKILL.md")
                .exists()
        );
        assert!(
            dst_dir
                .path()
                .join(".opencode/skills/review/SKILL.md")
                .exists()
        );
        assert!(
            dst_dir
                .path()
                .join(".agents/skills/review/SKILL.md")
                .exists()
        );
        assert!(
            dst_dir
                .path()
                .join(".cursor/skills/review/SKILL.md")
                .exists()
        );

        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn install_with_symlink_strategy() -> Result<()> {
        let src_dir = TempDir::new()?;
        let dst_dir = TempDir::new()?;

        // Create source command
        let cmd_dir = src_dir.path().join("commands");
        fs::create_dir_all(&cmd_dir)?;
        fs::write(cmd_dir.join("deploy.md"), "# Deploy")?;

        let manifest = make_manifest(vec![FileMapping {
            path: PathBuf::from("commands/deploy.md"),
            kind: FileKind::Command,
            strategy: FileStrategy::Link,
        }]);

        let results = install(
            &manifest,
            &[AgentProvider::ClaudeCode],
            &FileScope::Project,
            dst_dir.path(),
            src_dir.path(),
        )?;

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].strategy, FileStrategy::Link);

        let target = dst_dir.path().join(".claude/commands/deploy.md");
        assert!(target.is_symlink());
        assert_eq!(fs::read_to_string(&target)?, "# Deploy");

        Ok(())
    }

    #[test]
    fn missing_source_file_errors() {
        let src_dir = TempDir::new().unwrap();
        let dst_dir = TempDir::new().unwrap();

        let manifest = make_manifest(vec![FileMapping {
            path: PathBuf::from("nonexistent/SKILL.md"),
            kind: FileKind::Skill,
            strategy: FileStrategy::Copy,
        }]);

        let result = install(
            &manifest,
            &[AgentProvider::ClaudeCode],
            &FileScope::Project,
            dst_dir.path(),
            src_dir.path(),
        );

        assert!(result.is_err());
    }
}
