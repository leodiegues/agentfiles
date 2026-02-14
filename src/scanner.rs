use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use crate::manifest::FileMapping;
use crate::types::{AgentProvider, FileKind, FileStrategy};

/// Subdirectory names and their corresponding file kind.
const KIND_DIRS: &[(&str, FileKind)] = &[
    ("skills", FileKind::Skill),
    ("commands", FileKind::Command),
    ("agents", FileKind::Agent),
];

/// Scan a directory for agent files and return discovered file mappings.
///
/// Looks for files under known provider directories (e.g., `.claude/skills/`,
/// `.opencode/commands/`, `.agents/skills/`) as well as bare `skills/`,
/// `commands/`, and `agents/` directories at the root.
///
/// Skills are expected to be directories containing a `SKILL.md` file.
/// Commands and agents are expected to be `.md` files directly.
///
/// The returned `FileMapping` paths are relative to `root`.
pub fn scan_agent_files(root: &Path) -> Result<Vec<FileMapping>> {
    let root = root
        .canonicalize()
        .with_context(|| format!("cannot resolve path: {}", root.display()))?;

    let mut mappings = Vec::new();

    // Scan known provider-prefixed directories (derived from provider layouts)
    for prefix in AgentProvider::project_bases() {
        let prefix_dir = root.join(prefix);
        if prefix_dir.is_dir() {
            scan_kind_dirs(&root, &prefix_dir, &mut mappings)?;
        }
    }

    // Also scan bare kind directories at the root (e.g., ./skills/, ./commands/, ./agents/)
    for &(kind_name, ref kind) in KIND_DIRS {
        let kind_dir = root.join(kind_name);
        if kind_dir.is_dir() {
            scan_kind_dir(&root, &kind_dir, kind, &mut mappings)?;
        }
    }

    // Deduplicate by target filename (keep the first occurrence)
    deduplicate(&mut mappings);

    Ok(mappings)
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
/// - Skills: looks for `<name>/SKILL.md` subdirectories
/// - Commands/Agents: looks for `<name>.md` files
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
                // Skills are directories containing SKILL.md
                if entry_path.is_dir() {
                    let skill_md = entry_path.join("SKILL.md");
                    if skill_md.is_file() {
                        let rel_path = skill_md
                            .strip_prefix(root)
                            .unwrap_or(&skill_md)
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
                        kind: kind.clone(),
                        strategy: FileStrategy::Copy,
                    });
                }
            }
        }
    }

    Ok(())
}

/// Deduplicate file mappings by their filename stem + kind.
///
/// If the same skill/command/agent name appears from multiple provider
/// directories, keep only the first occurrence.
fn deduplicate(mappings: &mut Vec<FileMapping>) {
    let mut seen = std::collections::HashSet::new();
    mappings.retain(|m| {
        let key = format!(
            "{}:{}",
            m.kind,
            m.path
                .file_stem()
                .or_else(|| m.path.parent().and_then(|p| p.file_name()))
                .unwrap_or_default()
                .to_string_lossy()
        );
        seen.insert(key)
    });
}

/// Infer the folder name from a path to use as a manifest name.
pub fn infer_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "unnamed".to_string())
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

    #[test]
    fn scans_claude_skills() -> Result<()> {
        let dir = TempDir::new()?;
        setup_skill(dir.path(), ".claude", "review");

        let mappings = scan_agent_files(dir.path())?;
        assert_eq!(mappings.len(), 1);
        assert_eq!(mappings[0].kind, FileKind::Skill);
        assert!(mappings[0].path.ends_with("SKILL.md"));
        Ok(())
    }

    #[test]
    fn scans_multiple_providers() -> Result<()> {
        let dir = TempDir::new()?;
        setup_skill(dir.path(), ".claude", "review");
        setup_command(dir.path(), ".opencode", "deploy");
        setup_agent(dir.path(), ".cursor", "security");

        let mappings = scan_agent_files(dir.path())?;
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

        let mappings = scan_agent_files(dir.path())?;
        // Should deduplicate to 1
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

        let mappings = scan_agent_files(dir.path())?;
        assert_eq!(mappings.len(), 1);
        assert_eq!(mappings[0].kind, FileKind::Skill);
        Ok(())
    }

    #[test]
    fn empty_dir_returns_empty() -> Result<()> {
        let dir = TempDir::new()?;
        let mappings = scan_agent_files(dir.path())?;
        assert!(mappings.is_empty());
        Ok(())
    }

    #[test]
    fn infer_name_from_path() {
        assert_eq!(infer_name(Path::new("/home/user/my-project")), "my-project");
        assert_eq!(infer_name(Path::new("/")), "unnamed");
        assert_eq!(infer_name(Path::new("some-folder")), "some-folder");
    }
}
