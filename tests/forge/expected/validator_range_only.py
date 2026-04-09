# SCE Forge: Auto-generated from Extended SCXML (sce:kind="validator")
# Do not edit — regenerate from the source SCXML file.

from dataclasses import dataclass


@dataclass
class ValidationResult:
    valid: bool
    reason: str


class ValidatorRangeOnly:
    def __init__(self) -> None:
        pass

    def validate(self, temperature: float) -> ValidationResult:
        if temperature < -40.0 or temperature > 150.0:
            return ValidationResult(False, "temperature_out_of_range")
        return ValidationResult(True, "")