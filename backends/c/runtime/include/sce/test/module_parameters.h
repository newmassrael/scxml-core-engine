/* SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael */

/**
 * @file module_parameters.h
 * @brief The module parameters of a test run (docs/SCE_TEST_PROFILE.md §11.5,
 *        §11.8; ETSI ES 201 873-1 §8.2)
 *
 * The value a suite gives each module parameter, which every component that
 * declares a module parameter of that id reads into its variable before its
 * machine starts. The C11 port of `sce/include/test/ModuleParameters.h` and
 * `sce_rust_runtime::test::ModuleParameters`.
 *
 * A value is held as the one JSON value model a saved state is made of
 * (sce/saved_json.h), as its compact text. The generated suite writes each
 * default as the saved form of its declared type, and the generated component
 * reads it back through the reader of the type it declares (sce/saved_values.h),
 * which are the reads a restored variable is held by: a value of another type,
 * or past its bound, is refused by name rather than converted. The runtime knows
 * the type of no module parameter, so it is written once for every suite.
 *
 * The table holds at most `SCE_TEST_MAX_MODULE_PARAMETERS` parameters, and lends
 * each id and text rather than copying it: both are the caller's, a generated
 * suite's constant or a text the caller keeps for as long as a run reads it.
 * Header-only and allocation-free.
 */

#ifndef SCE_TEST_MODULE_PARAMETERS_H
#define SCE_TEST_MODULE_PARAMETERS_H

#include <stdbool.h>
#include <stddef.h>
#include <string.h>

#include "sce/saved_json.h"

#ifdef __cplusplus
extern "C" {
#endif

/* How many module parameters one suite declares. The profile's examples
   declare one; eight leaves room without reserving much of a run. A suite that
   declares more is given its parameters by a build that raises this
   (`-DSCE_TEST_MAX_MODULE_PARAMETERS=N`); a parameter past it is refused. */
#ifndef SCE_TEST_MAX_MODULE_PARAMETERS
#define SCE_TEST_MAX_MODULE_PARAMETERS 8u
#endif

/** One module parameter: its id and its value, the compact JSON text of the
 *  saved form of a value of its declared type. Both are lent. */
typedef struct sce_test_module_parameter_s {
    const char *id;
    const char *value;
} sce_test_module_parameter_t;

/**
 * The module parameters a run hands its components, by id.
 *
 * Generated from a `test-suite` with each parameter at its default, and given
 * other values by the caller of a run where it overrides one (-1 §8.2.1).
 */
typedef struct sce_test_module_parameters_s {
    sce_test_module_parameter_t entries[SCE_TEST_MAX_MODULE_PARAMETERS];
    size_t count;
} sce_test_module_parameters_t;

/** No module parameter. */
static inline void sce_test_module_parameters_init(sce_test_module_parameters_t *parameters) {
    memset(parameters, 0, sizeof(*parameters));
}

/** The entry of the module parameter `id`, or NULL when it has no value. */
static inline const sce_test_module_parameter_t *
sce_test_module_parameters_find(const sce_test_module_parameters_t *parameters, const char *id) {
    for (size_t i = 0u; i < parameters->count; ++i) {
        if (strcmp(parameters->entries[i].id, id) == 0) {
            return &parameters->entries[i];
        }
    }
    return NULL;
}

/**
 * Gives the module parameter `id` the value `value`, a JSON text, in place of
 * any it had. False, with the table as it was, when `id` has no value yet and
 * the table holds `SCE_TEST_MAX_MODULE_PARAMETERS` already.
 */
static inline bool sce_test_module_parameters_set(sce_test_module_parameters_t *parameters, const char *id,
                                                  const char *value) {
    for (size_t i = 0u; i < parameters->count; ++i) {
        if (strcmp(parameters->entries[i].id, id) == 0) {
            parameters->entries[i].value = value;
            return true;
        }
    }
    if (parameters->count >= SCE_TEST_MAX_MODULE_PARAMETERS) {
        return false;
    }
    parameters->entries[parameters->count].id = id;
    parameters->entries[parameters->count].value = value;
    parameters->count++;
    return true;
}

/** Whether the module parameter `id` has a value. */
static inline bool sce_test_module_parameters_has(const sce_test_module_parameters_t *parameters, const char *id) {
    return sce_test_module_parameters_find(parameters, id) != NULL;
}

/**
 * The value of the module parameter `id`, judged as one JSON value, in `*out`.
 *
 * Refused, naming it, when it has no value, since no value is made up in its
 * place, and when its text is no JSON value. `*out` is a span of the lent text.
 */
static inline bool sce_test_module_parameters_of(const sce_test_module_parameters_t *parameters, const char *id,
                                                 sce_jv_t *out, sce_saved_error_t *err) {
    const sce_test_module_parameter_t *found = sce_test_module_parameters_find(parameters, id);
    if (found == NULL) {
        return sce_saved_refuse(err, "no value is given for the module parameter '%s'", id);
    }
    sce_saved_error_t why;
    why.message[0] = '\0';
    if (!sce_json_parse(found->value, strlen(found->value), out, &why)) {
        return sce_saved_refuse(err, "the value of the module parameter '%s' is no JSON value: %s", id, why.message);
    }
    return true;
}

#ifdef __cplusplus
}
#endif

#endif /* SCE_TEST_MODULE_PARAMETERS_H */
