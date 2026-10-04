// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A 64-bit real in a typed event payload is written into the event's `data` the
// one way every engine writes a real (ARCHITECTURE.md, "JSON Number Text"):
// the fewest digits that read back, `5` for a whole value, and the exponent
// form from 1e21.
//
// The inject seam `..._raise_reading_taken_typed` writes the payload as the JSON
// text of the queued event's `data`, and the machine reads it back by `strtod`,
// so the machine behaves the same whichever digits were written: `0.1` and
// `0.10000000000000001` are one double. That is why this reads the TEXT, from the
// queue the seam wrote it into, and not what the machine did with it.
//
// MCU-clean acceptance: like its integer twin (test_event_schema_native.c) the
// target links NO runtime library, so a payload writer that needed one — or a
// header outside the runtime's include directory — would not link.
//
// Fixture: integration_resources/event_schema_real/event_schema_real.scxml
// (+ schema_reading.scxml), generated at CMake build time.

#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "event_schema_real_sm.h"

typedef struct {
    const char *name;
    uint64_t bits;
    const char *data;
} case_t;

// The data text of the one event the seam queued.
static const char *queued_data(const event_schema_real_t *sm) {
    return sm->external_queue.buf[sm->external_queue.head].data;
}

int main(void) {
    // Each value is its IEEE 754 bits, so no float parser decides it; the text is
    // the one tests/json_text/real_text.json states for the same bits.
    static const case_t cases[] = {
        {"a tenth is 0.1, not seventeen digits", UINT64_C(0x3FB999999999999A), "{\"value\":0.1}"},
        {"a whole value has no fraction", UINT64_C(0x4014000000000000), "{\"value\":5}"},
        {"1e21 takes the exponent form", UINT64_C(0x444B1AE4D6E2EF50), "{\"value\":1e+21}"},
        {"2 to the -44 is not seventeen digits", UINT64_C(0x3D30000000000000), "{\"value\":5.684341886080802e-14}"},
        {"negative zero is zero", UINT64_C(0x8000000000000000), "{\"value\":0}"},
    };
    int bad = 0;
    for (size_t i = 0u; i < sizeof(cases) / sizeof(cases[0]); i++) {
        event_schema_real_t sm;
        event_schema_real_init(&sm);
        event_schema_real_reading_taken_payload_t payload;
        memset(&payload, 0, sizeof(payload));
        memcpy(&payload.value, &cases[i].bits, sizeof(payload.value));
        if (!event_schema_real_raise_reading_taken_typed(&sm, &payload)) {
            (void)fprintf(stderr, "FAIL: %s: the seam refused a finite value\n", cases[i].name);
            bad = 1;
            continue;
        }
        if (strcmp(queued_data(&sm), cases[i].data) != 0) {
            (void)fprintf(stderr, "FAIL: %s: wrote %s, the table says %s\n", cases[i].name, queued_data(&sm),
                          cases[i].data);
            bad = 1;
        }
    }

    // JSON has no spelling for a value that is not finite, so the seam refuses it
    // rather than write text no peer can read.
    {
        event_schema_real_t sm;
        event_schema_real_init(&sm);
        event_schema_real_reading_taken_payload_t payload;
        memset(&payload, 0, sizeof(payload));
        payload.value = INFINITY;
        if (event_schema_real_raise_reading_taken_typed(&sm, &payload)) {
            (void)fprintf(stderr, "FAIL: the seam wrote a value that is not finite\n");
            bad = 1;
        }
    }

    if (bad != 0) {
        return 1;
    }
    (void)printf("PASS: C11 typed payload writes a real as ECMAScript spells it\n");
    return 0;
}
