# SCE-MAP: transform_bitwise:3 :: _forge_body

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="transform")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.


def compute_high_nibble(byte: int) -> int:
    return byte >> 4 & 0x0F


def compute_low_nibble(byte: int) -> int:
    return byte & 0x0F


# The name this module gives each part of the document's surface, keyed by the
# id the document wrote: the function computing each output, the parameter
# each input is passed as, and the field of the holder's record each output is
# returned in. A host driving the module by the document's ids reads the names
# here rather than re-deriving them, so it agrees with the generator whatever
# spelling the generator chose.
SCE_HOST_NAMES = {
    "functions": {
        "highNibble": "compute_high_nibble",
        "lowNibble": "compute_low_nibble",
    },
    "parameters": {
        "byte": "byte",
    },
    "fields": {
        "highNibble": "high_nibble",
        "lowNibble": "low_nibble",
    },
}
