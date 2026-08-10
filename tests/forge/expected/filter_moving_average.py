# SCE-MAP: filter_moving_average:1 :: _forge_body

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="filter")
# Runtime: sce_forge_runtime
# Do not edit — regenerate from the source SCXML file.

from sce_forge_runtime.filter import MovingAverage


class FilterMovingAverage:
    def __init__(self) -> None:
        self._impl = MovingAverage(window=5)

    def update(self, rawTemp: float) -> float:
        return self._impl.update(float(rawTemp))

    def reset(self) -> None:
        self._impl.reset()
