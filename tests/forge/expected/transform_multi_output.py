# SCE-MAP: transform_multi_output:3 :: _forge_body

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="transform")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.


def compute_fahrenheit(celsius: float) -> float:
    return celsius * 9 / 5 + 32


def compute_kelvin(celsius: float) -> float:
    return celsius + 273.15


# The name this module gives each part of the document's surface, keyed by the
# id the document wrote: the function computing each output, the parameter
# each input is passed as, and the field of the holder's record each output is
# returned in. A host driving the module by the document's ids reads the names
# here rather than re-deriving them, so it agrees with the generator whatever
# spelling the generator chose.
SCE_HOST_NAMES = {
    "functions": {
        "fahrenheit": "compute_fahrenheit",
        "kelvin": "compute_kelvin",
    },
    "parameters": {
        "celsius": "celsius",
    },
    "fields": {
        "fahrenheit": "fahrenheit",
        "kelvin": "kelvin",
    },
}
