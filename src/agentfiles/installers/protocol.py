from __future__ import annotations

from pathlib import Path
from typing import Protocol, runtime_checkable

from agentfiles.manifest import Manifest
from agentfiles.types import FileKind, FileScope


@runtime_checkable
class AgentFileInstaller(Protocol):
    name: str

    def get_target_dir(
        self, scope: FileScope, kind: FileKind, root: Path | None = None
    ) -> Path: ...
    def install(self, manifest: Manifest) -> Path: ...
