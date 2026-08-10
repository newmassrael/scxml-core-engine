// SCE-MAP: lookup_engine_status:3 :: _forge_body

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="lookup") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_LOOKUP_ENGINE_STATUS_H
#define SCE_FORGE_LOOKUP_ENGINE_STATUS_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

typedef enum {
    LOOKUP_ENGINE_STATUS_STATUS_STOP,
    LOOKUP_ENGINE_STATUS_STATUS_RUNNING,
    LOOKUP_ENGINE_STATUS_STATUS_FAULT
} lookup_engine_status_status_t;

static inline lookup_engine_status_status_t lookup_engine_status_status(uint8_t eng_sta) {
    switch (eng_sta) {
        case 0x07:
            return LOOKUP_ENGINE_STATUS_STATUS_FAULT;
        case 0x03:
            return LOOKUP_ENGINE_STATUS_STATUS_RUNNING;
        case 0x00:
        case 0x01:
        case 0x02:
            return LOOKUP_ENGINE_STATUS_STATUS_STOP;
        default: return LOOKUP_ENGINE_STATUS_STATUS_STOP;
    }
}

static inline const char *lookup_engine_status_status_name(lookup_engine_status_status_t v) {
    switch (v) {
        case LOOKUP_ENGINE_STATUS_STATUS_STOP: return "STOP";
        case LOOKUP_ENGINE_STATUS_STATUS_RUNNING: return "RUNNING";
        case LOOKUP_ENGINE_STATUS_STATUS_FAULT: return "FAULT";
    }
    return (const char *)0;
}

#endif  /* SCE_FORGE_LOOKUP_ENGINE_STATUS_H */
