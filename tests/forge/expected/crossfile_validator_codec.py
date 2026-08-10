# SCE-MAP: crossfile_validator_codec:4 :: _forge_body

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="validator")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from .codec_simple_frame import CodecSimpleFrame
from dataclasses import dataclass


@dataclass
class ValidationResult:
    valid: bool
    reason: str


class CrossfileValidatorCodec:
    def __init__(self) -> None:
        # Imported kinds (cross-file composition)
        self.frame: CodecSimpleFrame = CodecSimpleFrame()

    def validate(self, msg_id: int, payload: int) -> ValidationResult:
        if msg_id < 0 or msg_id > 255:
            return ValidationResult(False, "msg_id_out_of_range")
        if payload < 0 or payload > 4095:
            return ValidationResult(False, "payload_out_of_range")
        if not (self.frame.msg_id == msg_id and self.frame.payload == payload):
            return ValidationResult(False, "plausibility_failed")
        return ValidationResult(True, "")
