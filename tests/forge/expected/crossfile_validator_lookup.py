# SCE-MAP: crossfile_validator_lookup:7

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="validator")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from . import lookup_severity_default
from dataclasses import dataclass


@dataclass
class ValidationResult:
    valid: bool
    reason: str


class CrossfileValidatorLookup:
    def __init__(self) -> None:
        pass

    def validate(self, code: int) -> ValidationResult:
        if code < 0 or code > 1000:
            return ValidationResult(False, "code_out_of_range")
        if not (lookup_severity_default.lookup_severity(code) > 0):
            return ValidationResult(False, "plausibility_failed")
        return ValidationResult(True, "")
