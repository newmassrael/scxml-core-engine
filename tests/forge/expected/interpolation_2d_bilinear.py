# SCE-MAP: interpolation_2d_bilinear:1

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="interpolation")
# Runtime: sce_forge_runtime
# Do not edit — regenerate from the source SCXML file.

from sce_forge_runtime.interpolation import bilinear

AXIS_RPM = [800.0, 1200.0, 2000.0, 3000.0]
AXIS_LOAD = [10.0, 50.0, 100.0]
VALUES = [
    [2.1, 4.5, 7.0],
    [2.5, 5.0, 8.0],
    [3.0, 6.0, 9.5],
    [3.5, 7.0, 11.0],
]


def lookup(rpm: int, load: int) -> float:
    return bilinear(
        AXIS_RPM, AXIS_LOAD, VALUES,
        float(rpm), float(load))
