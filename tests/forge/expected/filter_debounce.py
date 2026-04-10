# SCE Forge: Auto-generated from Extended SCXML (sce:kind="filter")
# Do not edit — regenerate from the source SCXML file.

class FilterDebounce:
    def __init__(self) -> None:
        self._stable_value: bool = bool()
        self._candidate: bool = bool()
        self._count: int = 0
        self._initialized: bool = False

    def update(self, rawButton: bool) -> bool:
        value = rawButton
        if not self._initialized:
            self._stable_value = value
            self._candidate = value
            self._count = 1
            self._initialized = True
            return self._stable_value
        if value == self._candidate:
            self._count += 1
            if self._count >= 3:
                self._stable_value = self._candidate
        else:
            self._candidate = value
            self._count = 1
        return self._stable_value

    def reset(self) -> None:
        self._stable_value = bool()
        self._candidate = bool()
        self._count = 0
        self._initialized = False
