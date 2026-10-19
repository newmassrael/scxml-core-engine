/* SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael */

/**
 * @file action_realization.h
 * @brief What a host does with the duties of a machine (docs/adr/0038)
 *
 * A `<sce:action effect="duty">` is an operation the machine's correctness
 * depends on the host performing. The host table of a machine that states one
 * carries a `sce_action_realization_t` as its first member, and the machine
 * refuses a table that only simulates the duties unless it was initialised
 * through the entry the caller names for a simulated host
 * (`<machine>_init_with_simulated_actions`).
 *
 * `SCE_ACTION_PERFORMS` is a statement the host's author makes about the bodies
 * of the table's functions, in a line a reviewer can find. SCE cannot read the
 * bodies, and an empty body under `SCE_ACTION_PERFORMS` is that author's false
 * statement and not an unwritten fact. Header-only and allocation-free.
 */

#ifndef SCE_ACTION_REALIZATION_H
#define SCE_ACTION_REALIZATION_H

#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * What a host table does with the duties of the machine it is given to.
 *
 * `SCE_ACTION_UNSTATED` is the value a zero-initialised table holds. The other
 * backends make the member one the host must write (an interface member with no
 * default); a C table that never mentions it would otherwise read as
 * `SCE_ACTION_PERFORMS`, which is the one claim that must not be made by
 * leaving a line out. A machine that states a duty refuses an unstated table
 * exactly as it refuses one with a NULL function.
 */
typedef enum {
    SCE_ACTION_UNSTATED = 0, /**< the table says nothing: refused */
    SCE_ACTION_PERFORMS,     /**< the host performs the duties: its functions do what the names say */
    SCE_ACTION_SIMULATES     /**< the host records, drops or fakes them */
} sce_action_realization_t;

/**
 * @brief Whether a machine that states a duty takes a host table that says `realization`
 *
 * The one rule every generated initialiser applies, so the entries differ only
 * in the opt-in they pass: a table that performs is accepted; one that only
 * simulates is accepted only when the caller opted in by name; one that says
 * nothing, or says a value this type does not hold, is never accepted.
 */
static inline bool sce_action_realization_accepted(sce_action_realization_t realization, bool accept_simulated) {
    return realization == SCE_ACTION_PERFORMS || (realization == SCE_ACTION_SIMULATES && accept_simulated);
}

#ifdef __cplusplus
}
#endif

#endif /* SCE_ACTION_REALIZATION_H */
