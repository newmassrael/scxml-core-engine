# SCE Forge: Auto-generated from Extended SCXML (sce:kind="validator")
# Do not edit — regenerate from the source SCXML file.

from . import transform_temperature
from dataclasses import dataclass


@dataclass
class ValidationResult:
    valid: bool
    reason: str


class CrossfileValidatorTransform:
    def __init__(self) -> None:
        pass

    def validate(self, raw_temp: int) -> ValidationResult:
        if raw_temp < 0 or raw_temp > 4095:
            return ValidationResult(False, "raw_temp_out_of_range")
        if not (transform_temperature.compute_temperature(raw_temp) > -40.0 and transform_temperature.compute_temperature(raw_temp) < 200.0):
            return ValidationResult(False, "plausibility_failed")
        return ValidationResult(True, "")
