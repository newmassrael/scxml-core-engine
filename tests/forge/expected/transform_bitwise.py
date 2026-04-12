# SCE Forge: Auto-generated from Extended SCXML (sce:kind="transform")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.


def compute_high_nibble(byte: int) -> int:
    return byte >> 4 & 0x0F


def compute_low_nibble(byte: int) -> int:
    return byte & 0x0F
