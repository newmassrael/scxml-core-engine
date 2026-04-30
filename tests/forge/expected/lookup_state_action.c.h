/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="lookup") */
/* Runtime: sce_forge_runtime */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_LOOKUP_STATE_ACTION_H
#define SCE_FORGE_LOOKUP_STATE_ACTION_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

static const int32_t lookup_state_action_action_keys[4] = { 0, 1, 2, 3 };
static const int32_t lookup_state_action_action_values[4] = { 10, 20, 30, 40 };

static inline bool lookup_state_action_action(int32_t state, int32_t *out) {
    for (size_t _i = 0; _i < 4; ++_i) {
        if (lookup_state_action_action_keys[_i] == state) {
            *out = lookup_state_action_action_values[_i];
            return true;
        }
    }
    return false;
}

#endif  /* SCE_FORGE_LOOKUP_STATE_ACTION_H */
