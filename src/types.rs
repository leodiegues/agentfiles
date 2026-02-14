use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Scope determines where files are installed: relative to the project root or globally.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum FileScope {
    Project,
    Global,
}

impl fmt::Display for FileScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FileScope::Project => write!(f, "project"),
            FileScope::Global => write!(f, "global"),
        }
    }
}

impl FromStr for FileScope {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "project" => Ok(FileScope::Project),
            "global" => Ok(FileScope::Global),
            other => anyhow::bail!("unknown scope '{other}', expected 'project' or 'global'"),
        }
    }
}

/// The kind of agent file. Determines the target subdirectory.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum FileKind {
    Skill,
    Agent,
    Command,
}

impl fmt::Display for FileKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FileKind::Skill => write!(f, "Skill"),
            FileKind::Agent => write!(f, "Agent"),
            FileKind::Command => write!(f, "Command"),
        }
    }
}

/// How a file is placed at the target location.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub enum FileStrategy {
    #[default]
    Copy,
    Link,
}

impl fmt::Display for FileStrategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FileStrategy::Copy => write!(f, "copy"),
            FileStrategy::Link => write!(f, "link"),
        }
    }
}

impl FromStr for FileStrategy {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "copy" => Ok(FileStrategy::Copy),
            "link" | "symlink" => Ok(FileStrategy::Link),
            other => anyhow::bail!("unknown strategy '{other}', expected 'copy' or 'link'"),
        }
    }
}

/// Supported agentic coding tool providers.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum AgentProvider {
    ClaudeCode,
    OpenCode,
    Codex,
    Cursor,
}

impl AgentProvider {
    /// Returns all known providers.
    pub fn all() -> Vec<AgentProvider> {
        vec![
            AgentProvider::ClaudeCode,
            AgentProvider::OpenCode,
            AgentProvider::Codex,
            AgentProvider::Cursor,
        ]
    }
}

impl fmt::Display for AgentProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AgentProvider::ClaudeCode => write!(f, "Claude Code"),
            AgentProvider::OpenCode => write!(f, "OpenCode"),
            AgentProvider::Codex => write!(f, "Codex"),
            AgentProvider::Cursor => write!(f, "Cursor"),
        }
    }
}

impl FromStr for AgentProvider {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "claudecode" | "claude-code" | "claude_code" | "claude" => {
                Ok(AgentProvider::ClaudeCode)
            }
            "opencode" | "open-code" | "open_code" => Ok(AgentProvider::OpenCode),
            "codex" => Ok(AgentProvider::Codex),
            "cursor" => Ok(AgentProvider::Cursor),
            other => anyhow::bail!(
                "unknown provider '{other}', expected one of: claude-code, opencode, codex, cursor"
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_file_scope() {
        assert_eq!("project".parse::<FileScope>().unwrap(), FileScope::Project);
        assert_eq!("global".parse::<FileScope>().unwrap(), FileScope::Global);
        assert!("invalid".parse::<FileScope>().is_err());
    }

    #[test]
    fn parse_file_strategy() {
        assert_eq!("copy".parse::<FileStrategy>().unwrap(), FileStrategy::Copy);
        assert_eq!("link".parse::<FileStrategy>().unwrap(), FileStrategy::Link);
        assert_eq!(
            "symlink".parse::<FileStrategy>().unwrap(),
            FileStrategy::Link
        );
        assert!("invalid".parse::<FileStrategy>().is_err());
    }

    #[test]
    fn parse_agent_provider() {
        assert_eq!(
            "claude-code".parse::<AgentProvider>().unwrap(),
            AgentProvider::ClaudeCode
        );
        assert_eq!(
            "claude".parse::<AgentProvider>().unwrap(),
            AgentProvider::ClaudeCode
        );
        assert_eq!(
            "opencode".parse::<AgentProvider>().unwrap(),
            AgentProvider::OpenCode
        );
        assert_eq!(
            "codex".parse::<AgentProvider>().unwrap(),
            AgentProvider::Codex
        );
        assert_eq!(
            "cursor".parse::<AgentProvider>().unwrap(),
            AgentProvider::Cursor
        );
        assert!("invalid".parse::<AgentProvider>().is_err());
    }
}
