// SCE-MAP: crossfile_validator_codec:4 :: _forge_body

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="validator") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CROSSFILE_VALIDATOR_CODEC_H
#define SCE_FORGE_CROSSFILE_VALIDATOR_CODEC_H

#include <stdint.h>
#include <stdbool.h>
#include <string.h>
#include "codec_simple_frame.h"

typedef struct {
    bool        valid;
    const char *reason;
} crossfile_validator_codec_result_t;

typedef struct {
    /* Imported kind members (cross-file composition).
       Mirrors the procedure host's state struct layout — each stateful
       import (codec/filter/...) is embedded by-value so the rename
       map's `_st->{member}.{field}` expansion lands on a real slot and
       the C11 ImportLowering can prepend `&_st->{member}` for method
       dispatch (filter.update, future codec methods, ...). */
    codec_simple_frame_t frame_;
} crossfile_validator_codec_t;

static inline crossfile_validator_codec_result_t crossfile_validator_codec_validate(crossfile_validator_codec_t *_st, uint8_t msg_id, uint16_t payload) {
    if (payload > 4095)
        return (crossfile_validator_codec_result_t){false, "payload_out_of_range"};
    if (!(_st->frame_.msg_id == msg_id && _st->frame_.payload == payload))
        return (crossfile_validator_codec_result_t){false, "plausibility_failed"};
    return (crossfile_validator_codec_result_t){true, ""};
}

#endif  /* SCE_FORGE_CROSSFILE_VALIDATOR_CODEC_H */
