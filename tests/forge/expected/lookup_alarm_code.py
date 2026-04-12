# SCE Forge: Auto-generated from Extended SCXML (sce:kind="lookup")
# Runtime: sce_forge_runtime
# Do not edit — regenerate from the source SCXML file.

from typing import Optional

from sce_forge_runtime.lookup import lookup as _lookup

_KEYS = [100, 200, 300, 400, 500]
_VALUES = [1, 2, 3, 2, 4]


def lookup_severity(code: int) -> Optional[int]:
    return _lookup(_KEYS, _VALUES, code)
