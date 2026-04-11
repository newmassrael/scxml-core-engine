# SCE Forge: Auto-generated from Extended SCXML (sce:kind="filter")
# Do not edit — regenerate from the source SCXML file.

from sce_forge_runtime.filter import LowPass


class FilterLowPass:
    def __init__(self) -> None:
        self._impl = LowPass(alpha=0.1)

    def update(self, rawSignal: float) -> float:
        return self._impl.update(float(rawSignal))

    def reset(self) -> None:
        self._impl.reset()
