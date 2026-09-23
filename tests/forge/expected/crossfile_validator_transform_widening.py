# SCE-MAP: crossfile_validator_transform_widening:9 :: _forge_body

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="validator")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from . import transform_temperature
from dataclasses import dataclass


@dataclass
class ValidationResult:
    valid: bool
    reason: str


class CrossfileValidatorTransformWidening:
    def __init__(self) -> None:
        pass

    def validate(self, raw_byte: int) -> ValidationResult:
        if raw_byte > 200:
            return ValidationResult(False, "raw_byte_out_of_range")
        if not (transform_temperature.compute_temperature(raw_byte) > transform_temperature.compute_temperature(0)):
            return ValidationResult(False, "plausibility_failed")
        return ValidationResult(True, "")
