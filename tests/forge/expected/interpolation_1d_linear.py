# SCE Forge: Auto-generated from Extended SCXML (sce:kind="interpolation")
# Do not edit — regenerate from the source SCXML file.

AXIS_RPM = [800.0, 1200.0, 2000.0, 3000.0, 4000.0, 6000.0]
VALUES = [120.0, 145.0, 200.0, 230.0, 210.0, 180.0]


def lookup(rpm: int) -> float:
    return _linear_interpolate(
        AXIS_RPM, VALUES,
        float(rpm))


def _linear_interpolate(axis: list[float], values: list[float], x: float) -> float:
    n = len(axis)
    if x <= axis[0]:
        return values[0]
    if x >= axis[n - 1]:
        return values[n - 1]
    for i in range(n - 1):
        if x <= axis[i + 1]:
            t = (x - axis[i]) / (axis[i + 1] - axis[i])
            return values[i] + t * (values[i + 1] - values[i])
    return values[n - 1]
