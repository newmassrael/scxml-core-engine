/* SCE Forge: Auto-generated test-vector sidecar (RFC §5.B B2) */
/* Companion to algorithm_crc16.h — do not edit; regenerate from the source SCXML. */

#pragma once
#ifndef SCE_FORGE_ALGORITHM_CRC16_TEST_H
#define SCE_FORGE_ALGORITHM_CRC16_TEST_H

#include <cstdint>
#include <cstdio>
#include <span>
#include "algorithm_crc16.h"

static inline int test_vector_algorithm_crc16() {
    int failures = 0;
    {
        static constexpr std::uint8_t row_bytes[] = { 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39 };
        const uint16_t actual = SCE::Generated::AlgorithmCrc16::algorithm_crc16(std::span<const std::uint8_t>{row_bytes});
        const uint16_t expected = (uint16_t)0x29b1u;
        if (actual != expected) {
            std::fprintf(stderr,
                "FAIL: algorithm_crc16 test_vector @SCXML L47: actual=0x%llx expected=0x%llx\n",
                static_cast<unsigned long long>(actual), static_cast<unsigned long long>(expected));
            ++failures;
        }
    }
    return failures;
}

#endif  /* SCE_FORGE_ALGORITHM_CRC16_TEST_H */
