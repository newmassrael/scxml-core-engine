// SCE-MAP: procedure_linear:2

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="procedure") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_PROCEDURE_LINEAR_H
#define SCE_FORGE_PROCEDURE_LINEAR_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include <string.h>
/* L1 procedure: pure guard-only diamond. RFC §5.J.2 Phase D-1. */

typedef enum {
    PROCEDURE_LINEAR_STATE_STAGE_A = 0,
    PROCEDURE_LINEAR_STATE_STAGE_B = 1,
    PROCEDURE_LINEAR_STATE_STAGE_C = 2,
    PROCEDURE_LINEAR_STATE_DONE = 3
} procedure_linear_state_t;

typedef struct {
    bool         completed;
    const char  *final_state;
} procedure_linear_result_t;

static inline procedure_linear_result_t procedure_linear_execute(int32_t value) {
    /* Some L1 procedures (e.g. unconditional pipelines) ignore an input;
       silence -Wunused-parameter without losing the named slot in the
       public ABI. Emitting unconditionally keeps the template branch-free. */
    (void)value;
    procedure_linear_state_t current = PROCEDURE_LINEAR_STATE_STAGE_A;

    /* L1 safety cap mirrors the cpp run_procedure() bound. */
    for (int _iter = 0; _iter < 1000; ++_iter) {
        switch (current) {
            case PROCEDURE_LINEAR_STATE_STAGE_A:
                current = PROCEDURE_LINEAR_STATE_STAGE_B; break;
            case PROCEDURE_LINEAR_STATE_STAGE_B:
                current = PROCEDURE_LINEAR_STATE_STAGE_C; break;
            case PROCEDURE_LINEAR_STATE_STAGE_C:
                current = PROCEDURE_LINEAR_STATE_DONE; break;
            case PROCEDURE_LINEAR_STATE_DONE:
                return (procedure_linear_result_t){true, "done"};
            default:
                return (procedure_linear_result_t){false, ""};
        }
    }
    /* Safety cap exceeded — never expected for L1 acyclic guard flows. */
    return (procedure_linear_result_t){false, ""};
}

#endif  /* SCE_FORGE_PROCEDURE_LINEAR_H */
