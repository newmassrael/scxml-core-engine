# SCE Forge: Auto-generated from Extended SCXML (sce:kind="filter")
# Do not edit — regenerate from the source SCXML file.

class FilterLowPass:
    def __init__(self) -> None:
        self._prev: float = 0.0
        self._initialized: bool = False

    def update(self, rawSignal: float) -> float:
        if not self._initialized:
            self._prev = float(rawSignal)
            self._initialized = True
            return self._prev
        self._prev = 0.1 * float(rawSignal) + (1.0 - 0.1) * self._prev
        return self._prev

    def reset(self) -> None:
        self._prev = 0.0
        self._initialized = False
