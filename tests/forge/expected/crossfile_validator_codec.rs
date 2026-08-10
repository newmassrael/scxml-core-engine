#![doc = "SCE-MAP: crossfile_validator_codec:4 :: _forge_body"]
// SCE-MAP: crossfile_validator_codec:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="validator")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

use super::codec_simple_frame::CodecSimpleFrame;

// pub API: validators expose `ValidationResult` to downstream consumers
// (SCE_FORGE.md §7 validator). Fixtures that don't exercise the `reason`
// field would otherwise emit dead_code.
#[derive(Debug)]
#[allow(dead_code)]
pub struct ValidationResult {
    pub valid: bool,
    pub reason: String,
}

pub struct CrossfileValidatorCodec {
    // Imported kinds (cross-file composition)
    pub frame: CodecSimpleFrame,
}

impl CrossfileValidatorCodec {
    pub fn new() -> Self {
        Self {
            frame: CodecSimpleFrame::new(),
        }
    }

    pub fn validate(&mut self, msg_id: u8, payload: u16) -> ValidationResult {
        if payload > 4095 {
            return ValidationResult { valid: false, reason: "payload_out_of_range".to_string() };
        }
        if !(self.frame.msg_id == msg_id && self.frame.payload == payload) {
            return ValidationResult { valid: false, reason: "plausibility_failed".to_string() };
        }
        ValidationResult { valid: true, reason: String::new() }
    }
}
