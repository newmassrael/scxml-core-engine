/* SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael */

/*
 * A replay scenario as the `c11-mcu` gate's image runs it (docs/adr/0110, D3
 * item 3). The scenario's JSON is read when the gate generates the machine, by
 * `sce-codegen scenario-table`, which writes the one definition this declares:
 * `sce_mcu_scenario`, the machine's storage and start, and one row per step,
 * each the act the step makes (the event it sends in, the time it moves on) and
 * a judge that answers the first of its expectations that does not hold. The
 * runner (`scenario_runner.c`) walks the rows and writes the first mismatch; the
 * image's exit status is the verdict.
 *
 * Nothing here is read at run time but the table: the program opens no file and
 * parses no text, which an image with no C library could not do.
 */

#ifndef SCE_MCU_SCENARIO_H
#define SCE_MCU_SCENARIO_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include "sce_mcu_board.h"

/* Answer the expectation that failed, whose sentence is `literal`, from a judge. */
#define SCE_MCU_FAIL(literal)                                                                                          \
    do {                                                                                                               \
        static const sce_mcu_text_t sce_mcu_failed_ = SCE_MCU_TEXT(literal);                                           \
        return &sce_mcu_failed_;                                                                                       \
    } while (0)

/* One step of a scenario. */
typedef struct sce_mcu_step {
    /* `step <n>`, the step's place in the scenario, counted from one. */
    sce_mcu_text_t label;
    /* The event the step sends in and the time it moves on, each run to its end;
       NULL for a step that only expects. False when the act was refused. */
    bool (*act)(void *machine);
    /* NULL when every expectation of the step holds, else the sentence of the
       first that does not. */
    const sce_mcu_text_t *(*judge)(const void *machine);
} sce_mcu_step_t;

/* A scenario: the machine it drives, how it starts, and its steps. */
typedef struct sce_mcu_scenario {
    /* The machine's name, as the scenario gives it. */
    sce_mcu_text_t name;
    /* The machine's storage, static in the table's translation unit. */
    void *machine;
    /* Start the machine in its storage; false when it did not start. */
    bool (*start)(void *machine);
    const sce_mcu_step_t *steps;
    size_t steps_len;
    /* What a run that holds every step says: how many steps and expectations. */
    sce_mcu_text_t summary;
    /* The expectations the target does not judge, which the host C11 lane
       judges on the same scenario; an empty text when there are none. */
    sce_mcu_text_t unjudged;
} sce_mcu_scenario_t;

/* The scenario of this image, defined by the generated table. */
extern const sce_mcu_scenario_t sce_mcu_scenario;

/* Replay `scenario`: 0 when every step held, 1 at the first that did not, which
   is written through the log seam (`_sce_log_write`, sce/log.h), and 2 when the
   machine did not start or a step's act was refused. */
int sce_mcu_replay(const sce_mcu_scenario_t *scenario);

/* How many bits of `bits` are set: how many states an active-state bitmap holds. */
static inline size_t sce_mcu_bits(uint32_t bits) {
    size_t count = 0u;
    for (; bits != 0u; bits &= bits - 1u) {
        ++count;
    }
    return count;
}

/* Whether the text `got` ends after exactly the `len` bytes of `want`, each the
   same: a text a scenario states, compared by its length and its bytes. */
static inline bool sce_mcu_text_is(const char *got, const char *want, size_t len) {
    if (got == NULL) {
        return false;
    }
    for (size_t at = 0u; at < len; ++at) {
        if (got[at] != want[at]) {
            return false;
        }
    }
    return got[len] == '\0';
}

/* Whether the `got_len` bytes at `got` are the `len` bytes of `want`. */
static inline bool sce_mcu_bytes_are(const uint8_t *got, size_t got_len, const uint8_t *want, size_t len) {
    if (got_len != len) {
        return false;
    }
    for (size_t at = 0u; at < len; ++at) {
        if (got[at] != want[at]) {
            return false;
        }
    }
    return true;
}

/* The 64 bits of `value`: a real a scenario states is compared by them, exactly,
   so the image holds no formatter and no tolerance. */
static inline uint64_t sce_mcu_real_bits(double value) {
    union {
        double real;
        uint64_t bits;
    } both;

    _Static_assert(sizeof(double) == sizeof(uint64_t), "a double is 64 bits");
    both.real = value;
    return both.bits;
}

#endif /* SCE_MCU_SCENARIO_H */
