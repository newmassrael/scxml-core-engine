# SCE Forge: Auto-generated from Extended SCXML (sce:kind="interpolation")
# Do not edit — regenerate from the source SCXML file.

AXIS_RPM = [800.0, 1200.0, 2000.0, 3000.0]
AXIS_LOAD = [10.0, 50.0, 100.0]
VALUES = [
    [2.1, 4.5, 7.0],
    [2.5, 5.0, 8.0],
    [3.0, 6.0, 9.5],
    [3.5, 7.0, 11.0],
]


def lookup(rpm: int, load: int) -> float:
    return _bilinear_interpolate(
        AXIS_RPM, AXIS_LOAD, VALUES,
        float(rpm), float(load))


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


def _bilinear_interpolate(
        axis_x: list[float], axis_y: list[float],
        table: list[list[float]],
        x_in: float, y_in: float) -> float:
    x = max(axis_x[0], min(x_in, axis_x[-1]))
    y = max(axis_y[0], min(y_in, axis_y[-1]))
    ix, iy = 0, 0
    for i in range(len(axis_x) - 1):
        ix = i
        if x <= axis_x[i + 1]:
            break
    for i in range(len(axis_y) - 1):
        iy = i
        if y <= axis_y[i + 1]:
            break
    tx = (x - axis_x[ix]) / (axis_x[ix + 1] - axis_x[ix])
    ty = (y - axis_y[iy]) / (axis_y[iy + 1] - axis_y[iy])
    a = table[ix][iy] + tx * (table[ix + 1][iy] - table[ix][iy])
    b = table[ix][iy + 1] + tx * (table[ix + 1][iy + 1] - table[ix][iy + 1])
    return a + ty * (b - a)
