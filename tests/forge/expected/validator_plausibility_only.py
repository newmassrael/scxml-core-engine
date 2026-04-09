# SCE Forge: Auto-generated from Extended SCXML (sce:kind="validator")
# Do not edit — regenerate from the source SCXML file.

from dataclasses import dataclass


@dataclass
class ValidationResult:
    valid: bool
    reason: str


class ValidatorPlausibilityOnly:
    def __init__(self) -> None:
        pass

    def validate(self, voltage: float, current: float) -> ValidationResult:
        if not (voltage * current <= 1000.0):
            return ValidationResult(False, "plausibility_failed")
        return ValidationResult(True, "")