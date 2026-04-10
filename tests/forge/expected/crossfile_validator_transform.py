# SCE Forge: Auto-generated from Extended SCXML (sce:kind="validator")
# Do not edit — regenerate from the source SCXML file.

from .transform_temperature import TransformTemperature
from dataclasses import dataclass


@dataclass
class ValidationResult:
    valid: bool
    reason: str


class CrossfileValidatorTransform:
    def __init__(self) -> None:
        # Imported kinds (cross-file composition)

    def validate(self, raw_temp: int) -> ValidationResult:
        if raw_temp < 0 or raw_temp > 4095:
            return ValidationResult(False, "raw_temp_out_of_range")
        if not (compute_temperature(raw_temp) > -40 and compute_temperature(raw_temp) < 200):
            return ValidationResult(False, "plausibility_failed")
        return ValidationResult(True, "")