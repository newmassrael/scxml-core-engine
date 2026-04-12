# SCE Forge: Auto-generated from Extended SCXML (sce:kind="lookup")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from enum import Enum


class Quality(Enum):
    NONE = "NONE"
    LOW = "LOW"
    MEDIUM = "MEDIUM"
    HIGH = "HIGH"


def lookup_quality(level: int) -> Quality:
    if level == 3:
        return Quality.HIGH
    elif level == 1:
        return Quality.LOW
    elif level == 2:
        return Quality.MEDIUM
    elif level == 0:
        return Quality.NONE
    else:
        return Quality.NONE
