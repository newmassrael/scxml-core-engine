# SCE Forge: Auto-generated from Extended SCXML (sce:kind="lookup")
# Do not edit — regenerate from the source SCXML file.

from typing import Optional

from sce_forge_runtime.lookup import lookup as _lookup

_KEYS = [0, 1, 2, 3]
_VALUES = [10, 20, 30, 40]


def lookup_action(state: int) -> Optional[int]:
    return _lookup(_KEYS, _VALUES, state)
