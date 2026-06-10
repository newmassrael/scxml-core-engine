// SCE-MAP: procedure_startup_check:2

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="procedure") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_PROCEDURE_STARTUP_CHECK_H
#define SCE_FORGE_PROCEDURE_STARTUP_CHECK_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include <string.h>
/* L1 procedure: pure guard-only diamond (no externs, no helpers). */

typedef enum {
    PROCEDURE_STARTUP_CHECK_STATE_CHECK_VOLTAGE = 0,
    PROCEDURE_STARTUP_CHECK_STATE_CHECK_TEMP = 1,
    PROCEDURE_STARTUP_CHECK_STATE_SUCCESS = 2,
    PROCEDURE_STARTUP_CHECK_STATE_FAIL_VOLTAGE = 3,
    PROCEDURE_STARTUP_CHECK_STATE_FAIL_OVERTEMP = 4
} procedure_startup_check_state_t;

typedef struct {
    bool         completed;
    const char  *final_state;
} procedure_startup_check_result_t;

static inline procedure_startup_check_result_t procedure_startup_check_execute(float voltage, float temperature) {
    /* Some L1 procedures (e.g. unconditional pipelines) ignore an input;
       silence -Wunused-parameter without losing the named slot in the
       public ABI. Emitting unconditionally keeps the template branch-free. */
    (void)voltage;
    /* Some L1 procedures (e.g. unconditional pipelines) ignore an input;
       silence -Wunused-parameter without losing the named slot in the
       public ABI. Emitting unconditionally keeps the template branch-free. */
    (void)temperature;
    procedure_startup_check_state_t current = PROCEDURE_STARTUP_CHECK_STATE_CHECK_VOLTAGE;

    /* L1 safety cap mirrors the cpp run_procedure() bound. */
    for (int _iter = 0; _iter < 1000; ++_iter) {
        switch (current) {
            case PROCEDURE_STARTUP_CHECK_STATE_CHECK_VOLTAGE:
                if (voltage >= 11.5 && voltage <= 14.5) { current = PROCEDURE_STARTUP_CHECK_STATE_CHECK_TEMP; break; }
                current = PROCEDURE_STARTUP_CHECK_STATE_FAIL_VOLTAGE; break;
            case PROCEDURE_STARTUP_CHECK_STATE_CHECK_TEMP:
                if (temperature < 80.0) { current = PROCEDURE_STARTUP_CHECK_STATE_SUCCESS; break; }
                current = PROCEDURE_STARTUP_CHECK_STATE_FAIL_OVERTEMP; break;
            case PROCEDURE_STARTUP_CHECK_STATE_SUCCESS:
                return (procedure_startup_check_result_t){true, "success"};
            case PROCEDURE_STARTUP_CHECK_STATE_FAIL_VOLTAGE:
                return (procedure_startup_check_result_t){true, "fail_voltage"};
            case PROCEDURE_STARTUP_CHECK_STATE_FAIL_OVERTEMP:
                return (procedure_startup_check_result_t){true, "fail_overtemp"};
            default:
                return (procedure_startup_check_result_t){false, ""};
        }
    }
    /* Safety cap exceeded — never expected for L1 acyclic guard flows. */
    return (procedure_startup_check_result_t){false, ""};
}

#endif  /* SCE_FORGE_PROCEDURE_STARTUP_CHECK_H */
