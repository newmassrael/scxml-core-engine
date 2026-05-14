# SCE-MAP: validator_rpm_check:2

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="validator")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from dataclasses import dataclass


@dataclass
class ValidationResult:
    valid: bool
    reason: str


class ValidatorRpmCheck:
    def __init__(self) -> None:
        self.prev_rpm = 0

    def validate(self, rpm: int, engine_state: str) -> ValidationResult:
        if rpm < 0 or rpm > 8000:
            return ValidationResult(False, "rpm_out_of_range")
        delta = abs(rpm - self.prev_rpm)
        if delta > 500:
            return ValidationResult(False, "rpm_rate_of_change_exceeded")
        if not (rpm == 0 or engine_state != 'STOP'):
            return ValidationResult(False, "plausibility_failed")
        self.prev_rpm = rpm
        return ValidationResult(True, "")
