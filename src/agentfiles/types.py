from __future__ import annotations

from enum import Enum


class FileScope(str, Enum):
    GLOBAL = "global"
    LOCAL = "local"


class FileKind(str, Enum):
    SKILL = "skill"
    AGENT = "agent"
    COMMAND = "command"


class FileStrategy(str, Enum):
    COPY = "copy"
    LINK = "link"
