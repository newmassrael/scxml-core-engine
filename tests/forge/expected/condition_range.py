# SCE-MAP: condition_range:3

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="condition")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.


def condition_range(rpm: int, min_rpm: int, max_rpm: int) -> bool:
    return rpm >= min_rpm and rpm <= max_rpm
