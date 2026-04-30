/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="procedure") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_PROCEDURE_DIAMOND_H
#define SCE_FORGE_PROCEDURE_DIAMOND_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include <string.h>
/* L1 procedure: pure guard-only diamond. RFC §5.J.2 Phase D-1. */

typedef enum {
    PROCEDURE_DIAMOND_STATE_CLASSIFY = 0,
    PROCEDURE_DIAMOND_STATE_HIGH_PATH = 1,
    PROCEDURE_DIAMOND_STATE_MID_PATH = 2,
    PROCEDURE_DIAMOND_STATE_LOW_PATH = 3,
    PROCEDURE_DIAMOND_STATE_ACCEPT = 4,
    PROCEDURE_DIAMOND_STATE_REJECT = 5
} procedure_diamond_state_t;

typedef struct {
    bool         completed;
    const char  *final_state;
} procedure_diamond_result_t;

static inline procedure_diamond_result_t procedure_diamond_execute(uint16_t sensor_value, const char * mode) {
    /* Some L1 procedures (e.g. unconditional pipelines) ignore an input;
       silence -Wunused-parameter without losing the named slot in the
       public ABI. Emitting unconditionally keeps the template branch-free. */
    (void)sensor_value;
    /* Some L1 procedures (e.g. unconditional pipelines) ignore an input;
       silence -Wunused-parameter without losing the named slot in the
       public ABI. Emitting unconditionally keeps the template branch-free. */
    (void)mode;
    procedure_diamond_state_t current = PROCEDURE_DIAMOND_STATE_CLASSIFY;

    /* L1 safety cap mirrors the cpp run_procedure() bound. */
    for (int _iter = 0; _iter < 1000; ++_iter) {
        switch (current) {
            case PROCEDURE_DIAMOND_STATE_CLASSIFY:
                if (sensor_value > 1000) { current = PROCEDURE_DIAMOND_STATE_HIGH_PATH; break; }
                if (sensor_value > 500) { current = PROCEDURE_DIAMOND_STATE_MID_PATH; break; }
                current = PROCEDURE_DIAMOND_STATE_LOW_PATH; break;
            case PROCEDURE_DIAMOND_STATE_HIGH_PATH:
                if (strcmp(mode, "strict") == 0) { current = PROCEDURE_DIAMOND_STATE_REJECT; break; }
                current = PROCEDURE_DIAMOND_STATE_ACCEPT; break;
            case PROCEDURE_DIAMOND_STATE_MID_PATH:
                current = PROCEDURE_DIAMOND_STATE_ACCEPT; break;
            case PROCEDURE_DIAMOND_STATE_LOW_PATH:
                current = PROCEDURE_DIAMOND_STATE_ACCEPT; break;
            case PROCEDURE_DIAMOND_STATE_ACCEPT:
                return (procedure_diamond_result_t){true, "accept"};
            case PROCEDURE_DIAMOND_STATE_REJECT:
                return (procedure_diamond_result_t){true, "reject"};
            default:
                return (procedure_diamond_result_t){false, ""};
        }
    }
    /* Safety cap exceeded — never expected for L1 acyclic guard flows. */
    return (procedure_diamond_result_t){false, ""};
}

#endif  /* SCE_FORGE_PROCEDURE_DIAMOND_H */
