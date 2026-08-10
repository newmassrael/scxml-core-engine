// SCE-MAP: crossfile_validator_codec:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="validator")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CROSSFILE_VALIDATOR_CODEC_H
#define SCE_FORGE_CROSSFILE_VALIDATOR_CODEC_H

#include <cstdint>
#include <string>
#include "codec_simple_frame.h"

namespace SCE::Generated::CrossfileValidatorCodec {

struct ValidationResult {
    bool valid;
    std::string reason;
};

struct CrossfileValidatorCodec {

    // Imported kinds (cross-file composition)
    ::SCE::Generated::CodecSimpleFrame::CodecSimpleFrame frame_{};

    ValidationResult validate(uint8_t msgId, uint16_t payload) {
        if (payload > 4095)
            return {false, "payload_out_of_range"};
        if (!(frame_.msgId == msgId && frame_.payload == payload))
            return {false, "plausibility_failed"};
        return {true, ""};
    }
};

}  // namespace SCE::Generated::CrossfileValidatorCodec

#endif  // SCE_FORGE_CROSSFILE_VALIDATOR_CODEC_H
