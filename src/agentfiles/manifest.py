from __future__ import annotations

from pathlib import Path

from pydantic import BaseModel, Field

from agentfiles.types import FileKind


class FileMapping(BaseModel):
    kind: FileKind


class Manifest(BaseModel):
    name: str
    version: str
    description: str | None = None
    author: str | None = None
    repository: str | None = None
    files: list[FileMapping] = Field(default_factory=list)


def load_manifest(path: Path) -> Manifest:
    if not path.exists():
        raise FileNotFoundError(f"Manifest file not found at {path}")

    if path.is_dir():
        path = path / "agentfiles.json"

    return Manifest.model_validate(path.read_text(encoding="utf-8"))
