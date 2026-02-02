from __future__ import annotations

from pathlib import Path

from agentfiles.manifest import Manifest
from agentfiles.types import FileKind, FileScope

_PATHS = {
    FileKind.SKILL: ".claude/skills",
    FileKind.COMMAND: ".claude/commands",
    FileKind.AGENT: ".claude/agents",
}


class ClaudeCodeInstaller:
    name = "claude-code"

    def get_target_dir(
        self, scope: FileScope, kind: FileKind, project_root: Path | None = None
    ) -> Path:
        if scope == FileScope.GLOBAL:
            if project_root is not None:
                raise ValueError("project_root is not supported for global scope")
            return Path.home() / _PATHS[kind]
        return (project_root or Path.cwd()) / _PATHS[kind]

    def install(
        self, manifest: Manifest, scope: FileScope, project_root: Path | None = None
    ) -> Path:
        pass
