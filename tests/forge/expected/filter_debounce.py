# SCE-MAP: filter_debounce:1 :: _forge_body

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="filter")
# Runtime: sce_forge_runtime
# Do not edit — regenerate from the source SCXML file.

from sce_forge_runtime.filter import Debounce


class FilterDebounce:
    def __init__(self) -> None:
        self._impl = Debounce(window=3)

    def update(self, rawButton: bool) -> bool:
        return self._impl.update(bool(rawButton))

    def reset(self) -> None:
        self._impl.reset()
