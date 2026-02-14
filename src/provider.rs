use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::types::{AgentProvider, FileKind, FileScope};

/// Complete directory layout for a provider.
///
/// Defines the base directory prefix per scope and the subdirectory name
/// for each file kind. A `None` kind directory means the provider does
/// not support that kind.
///
/// Adding a new provider only requires adding one `ProviderLayout` to
/// `AgentProvider::layout()`. Everything else (compatibility matrix,
/// target dirs, scanner prefixes) derives from this.
struct ProviderLayout {
    /// Base directory for project-scope installations (e.g., ".claude").
    project_base: &'static str,
    /// Base directory for global-scope installations (e.g., ".config/opencode").
    global_base: &'static str,
    /// Subdirectory for skills, or None if unsupported.
    skills: Option<&'static str>,
    /// Subdirectory for commands, or None if unsupported.
    commands: Option<&'static str>,
    /// Subdirectory for agents, or None if unsupported.
    agents: Option<&'static str>,
}

impl ProviderLayout {
    /// Returns the subdirectory name for a given file kind, or None if unsupported.
    fn kind_dir(&self, kind: &FileKind) -> Option<&'static str> {
        match kind {
            FileKind::Skill => self.skills,
            FileKind::Command => self.commands,
            FileKind::Agent => self.agents,
        }
    }

    /// Returns the base directory for a given scope.
    fn base(&self, scope: &FileScope) -> &'static str {
        match scope {
            FileScope::Project => self.project_base,
            FileScope::Global => self.global_base,
        }
    }
}

impl AgentProvider {
    /// Returns the directory layout for this provider.
    ///
    /// This is the single source of truth for all provider directory
    /// structure and compatibility information.
    fn layout(&self) -> ProviderLayout {
        match self {
            AgentProvider::ClaudeCode => ProviderLayout {
                project_base: ".claude",
                global_base: ".claude",
                skills: Some("skills"),
                commands: Some("commands"),
                agents: Some("agents"),
            },
            AgentProvider::OpenCode => ProviderLayout {
                project_base: ".opencode",
                global_base: ".config/opencode",
                skills: Some("skills"),
                commands: Some("commands"),
                agents: Some("agents"),
            },
            AgentProvider::Codex => ProviderLayout {
                project_base: ".agents",
                global_base: ".agents",
                skills: Some("skills"),
                commands: None,
                agents: None,
            },
            AgentProvider::Cursor => ProviderLayout {
                project_base: ".cursor",
                global_base: ".cursor",
                skills: Some("skills"),
                commands: Some("commands"),
                agents: Some("agents"),
            },
        }
    }

    /// Whether this provider supports the given file kind.
    ///
    /// Derived from the provider layout — a kind is supported if its
    /// subdirectory is defined (not `None`).
    pub fn supports_kind(&self, kind: &FileKind) -> bool {
        self.layout().kind_dir(kind).is_some()
    }

    /// Returns the list of FileKinds this provider supports.
    pub fn supported_kinds(&self) -> Vec<FileKind> {
        [FileKind::Skill, FileKind::Agent, FileKind::Command]
            .into_iter()
            .filter(|k| self.supports_kind(k))
            .collect()
    }

    /// Returns the project-scope base directories for all providers.
    ///
    /// Used by the scanner to know which directory prefixes to look for
    /// when auto-discovering agent files.
    pub fn project_bases() -> Vec<&'static str> {
        let mut bases: Vec<&'static str> = AgentProvider::all()
            .iter()
            .map(|p| p.layout().project_base)
            .collect();
        bases.dedup();
        bases
    }

    /// Resolves the full target directory for a given scope and file kind.
    ///
    /// - **Project** scope: `<project_root>/<base>/<kind_dir>/`
    /// - **Global** scope: `$HOME/<base>/<kind_dir>/`
    ///
    /// Returns an error if the provider does not support the file kind,
    /// or if the home directory cannot be resolved for global scope.
    pub fn get_target_dir(
        &self,
        scope: &FileScope,
        kind: &FileKind,
        project_root: &Path,
    ) -> Result<PathBuf> {
        let layout = self.layout();
        let kind_dir = layout
            .kind_dir(kind)
            .with_context(|| format!("{self} does not support {kind} files"))?;

        let root = match scope {
            FileScope::Project => project_root.to_path_buf(),
            FileScope::Global => dirs::home_dir().context("could not determine home directory")?,
        };

        Ok(root.join(layout.base(scope)).join(kind_dir))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn claude_code_project_dirs() {
        let root = Path::new("/project");
        let p = AgentProvider::ClaudeCode;

        assert_eq!(
            p.get_target_dir(&FileScope::Project, &FileKind::Skill, root)
                .unwrap(),
            PathBuf::from("/project/.claude/skills")
        );
        assert_eq!(
            p.get_target_dir(&FileScope::Project, &FileKind::Command, root)
                .unwrap(),
            PathBuf::from("/project/.claude/commands")
        );
        assert_eq!(
            p.get_target_dir(&FileScope::Project, &FileKind::Agent, root)
                .unwrap(),
            PathBuf::from("/project/.claude/agents")
        );
    }

    #[test]
    fn opencode_project_dirs() {
        let root = Path::new("/project");
        let p = AgentProvider::OpenCode;

        assert_eq!(
            p.get_target_dir(&FileScope::Project, &FileKind::Skill, root)
                .unwrap(),
            PathBuf::from("/project/.opencode/skills")
        );
        assert_eq!(
            p.get_target_dir(&FileScope::Project, &FileKind::Command, root)
                .unwrap(),
            PathBuf::from("/project/.opencode/commands")
        );
        assert_eq!(
            p.get_target_dir(&FileScope::Project, &FileKind::Agent, root)
                .unwrap(),
            PathBuf::from("/project/.opencode/agents")
        );
    }

    #[test]
    fn codex_project_dirs() {
        let root = Path::new("/project");
        let p = AgentProvider::Codex;

        assert_eq!(
            p.get_target_dir(&FileScope::Project, &FileKind::Skill, root)
                .unwrap(),
            PathBuf::from("/project/.agents/skills")
        );

        // Codex does not support commands or agents
        assert!(p
            .get_target_dir(&FileScope::Project, &FileKind::Command, root)
            .is_err());
        assert!(p
            .get_target_dir(&FileScope::Project, &FileKind::Agent, root)
            .is_err());
    }

    #[test]
    fn cursor_project_dirs() {
        let root = Path::new("/project");
        let p = AgentProvider::Cursor;

        assert_eq!(
            p.get_target_dir(&FileScope::Project, &FileKind::Skill, root)
                .unwrap(),
            PathBuf::from("/project/.cursor/skills")
        );
        assert_eq!(
            p.get_target_dir(&FileScope::Project, &FileKind::Command, root)
                .unwrap(),
            PathBuf::from("/project/.cursor/commands")
        );
        assert_eq!(
            p.get_target_dir(&FileScope::Project, &FileKind::Agent, root)
                .unwrap(),
            PathBuf::from("/project/.cursor/agents")
        );
    }

    #[test]
    fn global_dirs_resolve_to_home() {
        let home = dirs::home_dir().expect("need $HOME for this test");
        let root = Path::new("/ignored");

        assert_eq!(
            AgentProvider::ClaudeCode
                .get_target_dir(&FileScope::Global, &FileKind::Skill, root)
                .unwrap(),
            home.join(".claude/skills")
        );
        assert_eq!(
            AgentProvider::OpenCode
                .get_target_dir(&FileScope::Global, &FileKind::Skill, root)
                .unwrap(),
            home.join(".config/opencode/skills")
        );
        assert_eq!(
            AgentProvider::Codex
                .get_target_dir(&FileScope::Global, &FileKind::Skill, root)
                .unwrap(),
            home.join(".agents/skills")
        );
        assert_eq!(
            AgentProvider::Cursor
                .get_target_dir(&FileScope::Global, &FileKind::Skill, root)
                .unwrap(),
            home.join(".cursor/skills")
        );
    }

    #[test]
    fn supports_kind_derived_from_layout() {
        // All providers support skills
        for provider in AgentProvider::all() {
            assert!(provider.supports_kind(&FileKind::Skill));
        }

        // Codex does NOT support commands or agents
        assert!(!AgentProvider::Codex.supports_kind(&FileKind::Command));
        assert!(!AgentProvider::Codex.supports_kind(&FileKind::Agent));

        // Others support all three
        for provider in [
            AgentProvider::ClaudeCode,
            AgentProvider::OpenCode,
            AgentProvider::Cursor,
        ] {
            assert!(provider.supports_kind(&FileKind::Command));
            assert!(provider.supports_kind(&FileKind::Agent));
        }
    }

    #[test]
    fn codex_supported_kinds() {
        let kinds = AgentProvider::Codex.supported_kinds();
        assert_eq!(kinds, vec![FileKind::Skill]);
    }

    #[test]
    fn project_bases_contains_all_providers() {
        let bases = AgentProvider::project_bases();
        assert!(bases.contains(&".claude"));
        assert!(bases.contains(&".opencode"));
        assert!(bases.contains(&".agents"));
        assert!(bases.contains(&".cursor"));
    }
}
