# SCE-MAP: lookup_unit_scale:6

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="lookup")
# Runtime: sce_forge_runtime
# Do not edit — regenerate from the source SCXML file.

from typing import Optional

from sce_forge_runtime.lookup import lookup as _lookup

_KEYS = [1, 2, 3, 4, 5, 6]
_VALUES = [0.001, 0.01, 0.1, 1.0, 10.0, 100.0]


def lookup_scale(unit: int) -> Optional[float]:
    return _lookup(_KEYS, _VALUES, unit)
