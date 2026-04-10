# SCE Forge: Auto-generated from Extended SCXML (sce:kind="filter")
# Do not edit — regenerate from the source SCXML file.

class FilterMovingAverage:
    def __init__(self) -> None:
        self._buffer: list[float] = [0.0] * 5
        self._index: int = 0
        self._filled: bool = False

    def update(self, rawTemp: float) -> float:
        self._buffer[self._index] = float(rawTemp)
        self._index = (self._index + 1) % 5
        if not self._filled and self._index == 0:
            self._filled = True
        count = 5 if self._filled else self._index
        return sum(self._buffer[:count]) / count

    def reset(self) -> None:
        self._buffer = [0.0] * 5
        self._index = 0
        self._filled = False
