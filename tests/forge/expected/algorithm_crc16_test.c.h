/* SCE Forge: Auto-generated test-vector sidecar (RFC §5.B B2) */
/* Companion to algorithm_crc16.h — do not edit; regenerate from the source SCXML. */

#ifndef SCE_FORGE_ALGORITHM_CRC16_TEST_H
#define SCE_FORGE_ALGORITHM_CRC16_TEST_H

#include <stdio.h>
#include <string.h>
#include "algorithm_crc16.h"

static inline int test_vector_algorithm_crc16(void) {
    int failures = 0;
    {
        sce_forge_bytes_t input = {0};
        static const uint8_t row_bytes[] = { 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39 };
        memcpy(input.data, row_bytes, sizeof row_bytes);
        input.len = sizeof row_bytes;
        uint16_t actual = algorithm_crc16((sce_forge_bytes_view_t){ input.data, input.len });
        const uint16_t expected = (uint16_t)0x29b1u;
        if (actual != expected) {
            fprintf(stderr,
                "FAIL: algorithm_crc16 test_vector @SCXML L47: actual=0x%llx expected=0x%llx\n",
                (unsigned long long)actual, (unsigned long long)expected);
            ++failures;
        }
    }
    return failures;
}

#endif  /* SCE_FORGE_ALGORITHM_CRC16_TEST_H */
