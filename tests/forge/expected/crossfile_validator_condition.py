# SCE-MAP: crossfile_validator_condition:3 :: _forge_body

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="validator")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from . import condition_threshold
from dataclasses import dataclass


@dataclass
class ValidationResult:
    valid: bool
    reason: str


class CrossfileValidatorCondition:
    def __init__(self) -> None:
        pass

    def validate(self, coolant_temp: float, oil_temp: float, max_temp: float) -> ValidationResult:
        if not (not condition_threshold.condition_threshold(coolant_temp, oil_temp, max_temp)):
            return ValidationResult(False, "plausibility_failed")
        return ValidationResult(True, "")
