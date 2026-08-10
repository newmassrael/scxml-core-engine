# SCE-MAP: crossfile_validator_filter:14 :: _forge_body

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="validator")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from .filter_low_pass import FilterLowPass
from dataclasses import dataclass


@dataclass
class ValidationResult:
    valid: bool
    reason: str


class CrossfileValidatorFilter:
    def __init__(self) -> None:
        # Imported kinds (cross-file composition)
        self.smoother: FilterLowPass = FilterLowPass()

    def validate(self, raw_sample: float, threshold: float) -> ValidationResult:
        if not (self.smoother.update(raw_sample) < threshold):
            return ValidationResult(False, "plausibility_failed")
        return ValidationResult(True, "")
