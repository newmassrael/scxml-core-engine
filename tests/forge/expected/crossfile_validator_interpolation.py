# SCE-MAP: crossfile_validator_interpolation:9

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="validator")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from . import interpolation_1d_linear
from dataclasses import dataclass


@dataclass
class ValidationResult:
    valid: bool
    reason: str


class CrossfileValidatorInterpolation:
    def __init__(self) -> None:
        pass

    def validate(self, rpm: int) -> ValidationResult:
        if rpm < 500 or rpm > 7000:
            return ValidationResult(False, "rpm_out_of_range")
        if not (interpolation_1d_linear.lookup(rpm) > 200.0):
            return ValidationResult(False, "plausibility_failed")
        return ValidationResult(True, "")
