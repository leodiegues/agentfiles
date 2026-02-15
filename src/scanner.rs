use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use crate::manifest::{FileMapping, PathMapping};
use crate::types::{AgentProvider, FileKind, FileStrategy};

/// Subdirectory names and their corresponding file kind.
const KIND_DIRS: &[(&str, FileKind)] = &[
    ("skills", FileKind::Skill),
    ("commands", FileKind::Command),
    ("agents", FileKind::Agent),
];

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Scan a directory for agent files and return discovered file mappings.
///
/// When `custom_paths` is `None`, uses the default convention: scans
/// provider-prefixed directories (`.claude/skills/`, `.opencode/commands/`, etc.)
/// and bare `skills/`, `commands/`, `agents/` directories at the root.
///
/// When `custom_paths` is `Some`, only scans the specified paths using their
/// declared kind. The default convention is entirely replaced.
///
/// Skills are directories containing a `SKILL.md` file (the whole directory
/// is recorded, not just the SKILL.md). Commands and agents are `.md` files.
///
/// The returned `FileMapping` paths are relative to `root`.
pub fn scan_agent_files(
    root: &Path,
    custom_paths: Option<&[PathMapping]>,
) -> Result<Vec<FileMapping>> {
    let root = root
        .canonicalize()
        .with_context(|| format!("cannot resolve path: {}", root.display()))?;

    let mut mappings = Vec::new();

    if let Some(paths) = custom_paths {
        scan_custom_paths(&root, paths, &mut mappings)?;
    } else {
        scan_default_convention(&root, &mut mappings)?;
    }

    deduplicate(&mut mappings);

    Ok(mappings)
}

/// Filter a list of file mappings by a pick list.
///
/// Pick items can be kind-prefixed (`"skills/review"`, `"commands/deploy"`)
/// or plain names (`"review"`). A plain name matches any kind.
pub fn filter_by_pick(mappings: Vec<FileMapping>, pick: &[String]) -> Vec<FileMapping> {
    mappings
        .into_iter()
        .filter(|m| {
            let name = m.path.file_stem().unwrap_or_default().to_string_lossy();

            pick.iter().any(|p| {
                if let Some((kind_prefix, pick_name)) = p.split_once('/') {
                    let kind_matches = match kind_prefix {
                        "skills" => m.kind == FileKind::Skill,
                        "commands" => m.kind == FileKind::Command,
                        "agents" => m.kind == FileKind::Agent,
                        _ => false,
                    };
                    kind_matches && name == pick_name
                } else {
                    name == p.as_str()
                }
            })
        })
        .collect()
}

/// Infer the folder name from a path to use as a manifest name.
pub fn infer_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "unnamed".to_string())
}

// ---------------------------------------------------------------------------
// Default convention scanning
// ---------------------------------------------------------------------------

/// Scan using the default convention: provider-prefixed dirs + bare kind dirs.
fn scan_default_convention(root: &Path, mappings: &mut Vec<FileMapping>) -> Result<()> {
    // Scan known provider-prefixed directories (derived from provider layouts)
    for prefix in AgentProvider::project_bases() {
        let prefix_dir = root.join(prefix);
        if prefix_dir.is_dir() {
            scan_kind_dirs(root, &prefix_dir, mappings)?;
        }
    }

    // Also scan bare kind directories at the root (e.g., ./skills/, ./commands/, ./agents/)
    for &(kind_name, ref kind) in KIND_DIRS {
        let kind_dir = root.join(kind_name);
        if kind_dir.is_dir() {
            scan_kind_dir(root, &kind_dir, kind, mappings)?;
        }
    }

    Ok(())
}

/// Scan subdirectories inside a provider prefix directory for skills/commands/agents.
fn scan_kind_dirs(root: &Path, prefix_dir: &Path, mappings: &mut Vec<FileMapping>) -> Result<()> {
    for &(kind_name, ref kind) in KIND_DIRS {
        let kind_dir = prefix_dir.join(kind_name);
        if kind_dir.is_dir() {
            scan_kind_dir(root, &kind_dir, kind, mappings)?;
        }
    }
    Ok(())
}

/// Scan a single kind directory for agent files.
///
/// - Skills: looks for `<name>/SKILL.md` subdirectories. Records the directory
///   path (not the SKILL.md), so the full skill directory is installed.
/// - Commands/Agents: looks for `<name>.md` files.
fn scan_kind_dir(
    root: &Path,
    kind_dir: &Path,
    kind: &FileKind,
    mappings: &mut Vec<FileMapping>,
) -> Result<()> {
    let entries = fs::read_dir(kind_dir)
        .with_context(|| format!("cannot read directory: {}", kind_dir.display()))?;

    for entry in entries {
        let entry = entry?;
        let entry_path = entry.path();

        match kind {
            FileKind::Skill => {
                // Skills are directories containing SKILL.md.
                // We record the directory path so the entire skill dir is installed.
                if entry_path.is_dir() {
                    let skill_md = entry_path.join("SKILL.md");
                    if skill_md.is_file() {
                        let rel_path = entry_path
                            .strip_prefix(root)
                            .unwrap_or(&entry_path)
                            .to_path_buf();
                        mappings.push(FileMapping {
                            path: rel_path,
                            kind: FileKind::Skill,
                            strategy: FileStrategy::Copy,
                        });
                    }
                }
            }
            FileKind::Command | FileKind::Agent => {
                // Commands and agents are .md files
                if entry_path.is_file()
                    && let Some(ext) = entry_path.extension()
                    && ext == "md"
                {
                    let rel_path = entry_path
                        .strip_prefix(root)
                        .unwrap_or(&entry_path)
                        .to_path_buf();
                    mappings.push(FileMapping {
                        path: rel_path,
                        kind: *kind,
                        strategy: FileStrategy::Copy,
                    });
                }
            }
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Custom path scanning
// ---------------------------------------------------------------------------

/// Scan custom path mappings. Each entry maps a relative path to a file kind.
/// Directories are scanned using the standard kind convention. Files are
/// added directly.
fn scan_custom_paths(
    root: &Path,
    paths: &[PathMapping],
    mappings: &mut Vec<FileMapping>,
) -> Result<()> {
    for mapping in paths {
        let full_path = root.join(&mapping.path);

        if !full_path.exists() {
            // Skip paths that don't exist -- the source may not have all
            // configured paths.
            continue;
        }

        if full_path.is_dir() {
            // Scan directory using the standard convention for this kind
            scan_kind_dir(root, &full_path, &mapping.kind, mappings)?;
        } else if full_path.is_file() {
            // Single file -- add directly
            let rel_path = full_path
                .strip_prefix(root)
                .unwrap_or(&full_path)
                .to_path_buf();
            mappings.push(FileMapping {
                path: rel_path,
                kind: mapping.kind,
                strategy: FileStrategy::Copy,
            });
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Deduplication
// ---------------------------------------------------------------------------

/// Deduplicate file mappings by their name + kind.
///
/// If the same skill/command/agent name appears from multiple provider
/// directories, keep only the first occurrence.
fn deduplicate(mappings: &mut Vec<FileMapping>) {
    let mut seen = std::collections::HashSet::new();
    mappings.retain(|m| {
        let stem = m.path.file_stem().unwrap_or_default().to_string_lossy();
        let key = format!("{}:{}", m.kind, stem);
        seen.insert(key)
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_skill(dir: &Path, prefix: &str, name: &str) {
        let skill_dir = dir.join(prefix).join("skills").join(name);
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(
            skill_dir.join("SKILL.md"),
            format!("---\nname: {name}\ndescription: test\n---\nTest skill"),
        )
        .unwrap();
    }

    fn setup_command(dir: &Path, prefix: &str, name: &str) {
        let cmd_dir = dir.join(prefix).join("commands");
        fs::create_dir_all(&cmd_dir).unwrap();
        fs::write(
            cmd_dir.join(format!("{name}.md")),
            format!("---\ndescription: test\n---\nTest command"),
        )
        .unwrap();
    }

    fn setup_agent(dir: &Path, prefix: &str, name: &str) {
        let agent_dir = dir.join(prefix).join("agents");
        fs::create_dir_all(&agent_dir).unwrap();
        fs::write(
            agent_dir.join(format!("{name}.md")),
            format!("---\ndescription: test\n---\nTest agent"),
        )
        .unwrap();
    }

    // -----------------------------------------------------------------------
    // Default convention tests
    // -----------------------------------------------------------------------

    #[test]
    fn scans_claude_skills() -> Result<()> {
        let dir = TempDir::new()?;
        setup_skill(dir.path(), ".claude", "review");

        let mappings = scan_agent_files(dir.path(), None)?;
        assert_eq!(mappings.len(), 1);
        assert_eq!(mappings[0].kind, FileKind::Skill);
        // Should store the directory path, not SKILL.md
        assert!(mappings[0].path.ends_with("review"));
        assert!(!mappings[0].path.to_string_lossy().contains("SKILL.md"));
        Ok(())
    }

    #[test]
    fn scans_multiple_providers() -> Result<()> {
        let dir = TempDir::new()?;
        setup_skill(dir.path(), ".claude", "review");
        setup_command(dir.path(), ".opencode", "deploy");
        setup_agent(dir.path(), ".cursor", "security");

        let mappings = scan_agent_files(dir.path(), None)?;
        assert_eq!(mappings.len(), 3);

        let kinds: Vec<&FileKind> = mappings.iter().map(|m| &m.kind).collect();
        assert!(kinds.contains(&&FileKind::Skill));
        assert!(kinds.contains(&&FileKind::Command));
        assert!(kinds.contains(&&FileKind::Agent));
        Ok(())
    }

    #[test]
    fn deduplicates_same_skill_across_providers() -> Result<()> {
        let dir = TempDir::new()?;
        setup_skill(dir.path(), ".claude", "review");
        setup_skill(dir.path(), ".opencode", "review");

        let mappings = scan_agent_files(dir.path(), None)?;
        let skills: Vec<_> = mappings
            .iter()
            .filter(|m| m.kind == FileKind::Skill)
            .collect();
        assert_eq!(skills.len(), 1);
        Ok(())
    }

    #[test]
    fn scans_bare_directories() -> Result<()> {
        let dir = TempDir::new()?;
        // Create bare skills/ directory (not under a provider prefix)
        setup_skill(dir.path(), "", "my-skill");

        let mappings = scan_agent_files(dir.path(), None)?;
        assert_eq!(mappings.len(), 1);
        assert_eq!(mappings[0].kind, FileKind::Skill);
        Ok(())
    }

    #[test]
    fn skill_stores_directory_path() -> Result<()> {
        let dir = TempDir::new()?;
        // Create a skill with extra files alongside SKILL.md
        let skill_dir = dir.path().join("skills").join("my-skill");
        fs::create_dir_all(&skill_dir)?;
        fs::write(skill_dir.join("SKILL.md"), "# My Skill")?;
        fs::write(skill_dir.join("helper.py"), "# helper script")?;
        fs::create_dir_all(skill_dir.join("templates"))?;
        fs::write(skill_dir.join("templates/base.html"), "<html>")?;

        let mappings = scan_agent_files(dir.path(), None)?;
        assert_eq!(mappings.len(), 1);
        // The path should be the directory, not SKILL.md
        assert_eq!(
            mappings[0].path,
            std::path::PathBuf::from("skills/my-skill")
        );
        Ok(())
    }

    #[test]
    fn empty_dir_returns_empty() -> Result<()> {
        let dir = TempDir::new()?;
        let mappings = scan_agent_files(dir.path(), None)?;
        assert!(mappings.is_empty());
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Custom path tests
    // -----------------------------------------------------------------------

    #[test]
    fn custom_paths_scan_directory() -> Result<()> {
        let dir = TempDir::new()?;
        // Create skills in a non-standard directory
        let prompts_dir = dir.path().join("prompts").join("my-skill");
        fs::create_dir_all(&prompts_dir)?;
        fs::write(prompts_dir.join("SKILL.md"), "# My Skill")?;

        let custom = vec![PathMapping {
            path: "prompts".to_string(),
            kind: FileKind::Skill,
        }];

        let mappings = scan_agent_files(dir.path(), Some(&custom))?;
        assert_eq!(mappings.len(), 1);
        assert_eq!(mappings[0].kind, FileKind::Skill);
        assert!(mappings[0].path.to_string_lossy().contains("my-skill"));
        Ok(())
    }

    #[test]
    fn custom_paths_scan_single_file() -> Result<()> {
        let dir = TempDir::new()?;
        fs::write(dir.path().join("GUIDELINES.md"), "# Coding Guidelines")?;

        let custom = vec![PathMapping {
            path: "GUIDELINES.md".to_string(),
            kind: FileKind::Skill,
        }];

        let mappings = scan_agent_files(dir.path(), Some(&custom))?;
        assert_eq!(mappings.len(), 1);
        assert_eq!(mappings[0].kind, FileKind::Skill);
        assert_eq!(mappings[0].path, std::path::PathBuf::from("GUIDELINES.md"));
        Ok(())
    }

    #[test]
    fn custom_paths_replaces_defaults() -> Result<()> {
        let dir = TempDir::new()?;
        // Create standard skills/ dir AND a custom prompts/ dir
        setup_skill(dir.path(), "", "standard-skill");
        let prompts_dir = dir.path().join("prompts").join("custom-skill");
        fs::create_dir_all(&prompts_dir)?;
        fs::write(prompts_dir.join("SKILL.md"), "# Custom")?;

        // With custom paths, only prompts/ should be scanned
        let custom = vec![PathMapping {
            path: "prompts".to_string(),
            kind: FileKind::Skill,
        }];

        let mappings = scan_agent_files(dir.path(), Some(&custom))?;
        assert_eq!(mappings.len(), 1);
        assert!(mappings[0].path.to_string_lossy().contains("custom-skill"));
        Ok(())
    }

    #[test]
    fn custom_paths_skips_missing() -> Result<()> {
        let dir = TempDir::new()?;
        let custom = vec![PathMapping {
            path: "nonexistent".to_string(),
            kind: FileKind::Skill,
        }];

        let mappings = scan_agent_files(dir.path(), Some(&custom))?;
        assert!(mappings.is_empty());
        Ok(())
    }

    #[test]
    fn custom_paths_nested_directory() -> Result<()> {
        let dir = TempDir::new()?;
        let nested = dir
            .path()
            .join("src")
            .join("ai")
            .join("prompts")
            .join("review");
        fs::create_dir_all(&nested)?;
        fs::write(nested.join("SKILL.md"), "# Review")?;

        let custom = vec![PathMapping {
            path: "src/ai/prompts".to_string(),
            kind: FileKind::Skill,
        }];

        let mappings = scan_agent_files(dir.path(), Some(&custom))?;
        assert_eq!(mappings.len(), 1);
        assert_eq!(mappings[0].kind, FileKind::Skill);
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Pick filter tests
    // -----------------------------------------------------------------------

    #[test]
    fn filter_by_plain_name() {
        let mappings = vec![
            FileMapping {
                path: "skills/review".into(),
                kind: FileKind::Skill,
                strategy: FileStrategy::Copy,
            },
            FileMapping {
                path: "skills/deploy".into(),
                kind: FileKind::Skill,
                strategy: FileStrategy::Copy,
            },
            FileMapping {
                path: "commands/deploy.md".into(),
                kind: FileKind::Command,
                strategy: FileStrategy::Copy,
            },
        ];

        let filtered = filter_by_pick(mappings, &["review".to_string()]);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].kind, FileKind::Skill);
    }

    #[test]
    fn filter_by_kind_prefix() {
        let mappings = vec![
            FileMapping {
                path: "skills/deploy".into(),
                kind: FileKind::Skill,
                strategy: FileStrategy::Copy,
            },
            FileMapping {
                path: "commands/deploy.md".into(),
                kind: FileKind::Command,
                strategy: FileStrategy::Copy,
            },
        ];

        let filtered = filter_by_pick(mappings, &["commands/deploy".to_string()]);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].kind, FileKind::Command);
    }

    #[test]
    fn filter_plain_name_matches_all_kinds() {
        let mappings = vec![
            FileMapping {
                path: "skills/deploy".into(),
                kind: FileKind::Skill,
                strategy: FileStrategy::Copy,
            },
            FileMapping {
                path: "commands/deploy.md".into(),
                kind: FileKind::Command,
                strategy: FileStrategy::Copy,
            },
        ];

        let filtered = filter_by_pick(mappings, &["deploy".to_string()]);
        assert_eq!(filtered.len(), 2);
    }

    // -----------------------------------------------------------------------
    // Infer name
    // -----------------------------------------------------------------------

    #[test]
    fn infer_name_from_path() {
        assert_eq!(infer_name(Path::new("/home/user/my-project")), "my-project");
        assert_eq!(infer_name(Path::new("/")), "unnamed");
        assert_eq!(infer_name(Path::new("some-folder")), "some-folder");
    }
}
