/* SCE Forge: Auto-generated test-vector sidecar (RFC §synth-5-B B2) */
/* Companion to algorithm_crc16.h — do not edit; regenerate from the source SCXML. */

#ifndef SCE_FORGE_ALGORITHM_CRC16_TEST_H
#define SCE_FORGE_ALGORITHM_CRC16_TEST_H

#include <stdint.h>
#include "sce/text.h"
#include "sce/forge/test_vector.h"
#include "algorithm_crc16.h"

static inline int test_vector_algorithm_crc16(void) {
    int failures = 0;
    /* The one line every failed row's report is built in, in turn. */
    char _line[SCE_FORGE_TEST_VECTOR_LINE_CAP];
    {
        sce_forge_bytes_t input = {0};
        static const uint8_t row_bytes[] = { 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39 };
        memcpy(input.data, row_bytes, sizeof row_bytes);
        input.len = sizeof row_bytes;
        uint16_t actual = algorithm_crc16((sce_forge_bytes_view_t){ input.data, input.len });
        const uint16_t expected = (uint16_t)0x29b1u;
        if (actual != expected) {
            sce_text_writer_t _report = sce_forge_test_vector_line(_line, sizeof _line, 47u);
            SCE_TEXT_PUT_LITERAL(&_report, "actual=0x");
            sce_text_put_hex(&_report, (uint64_t)actual, 0u);
            SCE_TEXT_PUT_LITERAL(&_report, " expected=0x");
            sce_text_put_hex(&_report, (uint64_t)expected, 0u);
            sce_forge_test_vector_failed("algorithm_crc16", &_report);
            ++failures;
        }
    }
    return failures;
}

#endif  /* SCE_FORGE_ALGORITHM_CRC16_TEST_H */
