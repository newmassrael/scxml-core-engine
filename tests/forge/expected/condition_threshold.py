# SCE-MAP: condition_threshold:3 :: _forge_body

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="condition")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.


def condition_threshold(coolant_temp: float, oil_temp: float, max_temp: float) -> bool:
    return coolant_temp > max_temp or oil_temp > max_temp
