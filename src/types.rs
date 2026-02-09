use std::path::PathBuf;

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum FileScope {
    Project,
    Global,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum FileKind {
    Skill,
    Agent,
    Command,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum AgentProvider {
    ClaudeCode,
    Cursor,
    VSCode,
    Codex,
}

impl AgentProvider {
    fn subdir(&self, kind: &FileKind) -> &'static str {
        match (self, kind) {
            (AgentProvider::ClaudeCode, FileKind::Agent) => ".claude/agents",
            (AgentProvider::ClaudeCode, FileKind::Command) => ".claude/commands",
            (AgentProvider::ClaudeCode, FileKind::Skill) => ".claude/skills",
            (AgentProvider::Cursor, FileKind::Agent) => ".cursor/agents",
            (AgentProvider::Cursor, FileKind::Command) => ".cursor/commands",
            (AgentProvider::Cursor, FileKind::Skill) => ".cursor/skills",
            (AgentProvider::VSCode, FileKind::Agent) => ".vscode/agents",
            (AgentProvider::VSCode, FileKind::Command) => ".vscode/commands",
            (AgentProvider::VSCode, FileKind::Skill) => ".vscode/skills",
            (AgentProvider::Codex, FileKind::Agent) => ".codex/agents",
            (AgentProvider::Codex, FileKind::Command) => ".codex/commands",
            (AgentProvider::Codex, FileKind::Skill) => ".codex/skills",
        }
    }

    fn get_target_dir(&self, scope: &FileScope, kind: &FileKind) -> Result<PathBuf> {
        let mut target_dir = match scope {
            FileScope::Project => PathBuf::from("."),
            FileScope::Global => PathBuf::from("~"),
        };

        target_dir.push(self.subdir(kind));

        Ok(target_dir)
    }

    pub fn install(&self, scope: &FileScope, kind: &FileKind) -> Result<()> {
        let target_dir = self.get_target_dir(scope, kind)?;
        std::fs::create_dir_all(&target_dir)?;

        Ok(())
    }
}
