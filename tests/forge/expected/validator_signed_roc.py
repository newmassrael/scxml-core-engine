# SCE Forge: Auto-generated from Extended SCXML (sce:kind="validator")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from dataclasses import dataclass


@dataclass
class ValidationResult:
    valid: bool
    reason: str


class ValidatorSignedRoc:
    def __init__(self) -> None:
        self.prev_speed = 0
        self.prev_altitude = 0.0

    def validate(self, speed: int, altitude: float) -> ValidationResult:
        if speed < -100 or speed > 500:
            return ValidationResult(False, "speed_out_of_range")
        if altitude > 50000.0:
            return ValidationResult(False, "altitude_out_of_range")
        delta = abs(speed - self.prev_speed)
        if delta > 50:
            return ValidationResult(False, "speed_rate_of_change_exceeded")
        delta = abs(altitude - self.prev_altitude)
        if delta > 100.0:
            return ValidationResult(False, "altitude_rate_of_change_exceeded")
        self.prev_speed = speed
        self.prev_altitude = altitude
        return ValidationResult(True, "")
