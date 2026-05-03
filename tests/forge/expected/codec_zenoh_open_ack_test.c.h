/* SCE Forge: Auto-generated codec test-vector sidecar (RFC §5.B B5-θ) */
/* Companion to codec_zenoh_open_ack.h — do not edit; regenerate from the source SCXML. */

#ifndef SCE_FORGE_CODEC_ZENOH_OPEN_ACK_TEST_H
#define SCE_FORGE_CODEC_ZENOH_OPEN_ACK_TEST_H

#include <stdio.h>
#include <string.h>
#include "codec_zenoh_open_ack.h"

static inline int test_vector_codec_zenoh_open_ack(void) {
    int failures = 0;
    {
        codec_zenoh_open_ack_t actual = {0};
        actual.lease = (uint64_t)0x0uLL;
        actual.initial_sn = (uint64_t)0x0uLL;
        codec_zenoh_open_ack_encoded_t encoded = codec_zenoh_open_ack_encode(&actual);
        static const uint8_t _expected[] = { 0x00, 0x00 };
        if (encoded.len != sizeof _expected
            || memcmp(encoded.bytes, _expected, encoded.len) != 0) {
            fprintf(stderr,
                "FAIL: codec_zenoh_open_ack test_vector @SCXML L33: "
                "encode length=%zu (expected %zu)\n",
                encoded.len, sizeof _expected);
            ++failures;
        }
        sce_forge_cursor_t _cursor = sce_forge_cursor_init(_expected, sizeof _expected);
        codec_zenoh_open_ack_t decoded = {0};
        sce_forge_codec_status_t _st = codec_zenoh_open_ack_decode(&_cursor, &decoded);
        if (_st != SCE_FORGE_CODEC_OK) {
            fprintf(stderr,
                "FAIL: codec_zenoh_open_ack test_vector @SCXML L33: "
                "decode status=%d\n", (int)_st);
            ++failures;
        } else if (sce_forge_cursor_remaining(&_cursor) != 0) {
            fprintf(stderr,
                "FAIL: codec_zenoh_open_ack test_vector @SCXML L33: "
                "decode left %zu bytes unconsumed\n",
                sce_forge_cursor_remaining(&_cursor));
            ++failures;
        } else {
            if (decoded.lease != (uint64_t)0x0uLL) {
                fprintf(stderr,
                    "FAIL: codec_zenoh_open_ack test_vector @SCXML L33: "
                    "field `lease` mismatch\n");
                ++failures;
            }
            if (decoded.initial_sn != (uint64_t)0x0uLL) {
                fprintf(stderr,
                    "FAIL: codec_zenoh_open_ack test_vector @SCXML L33: "
                    "field `initial_sn` mismatch\n");
                ++failures;
            }
        }
    }
    {
        codec_zenoh_open_ack_t actual = {0};
        actual.lease = (uint64_t)0x1uLL;
        actual.initial_sn = (uint64_t)0x64uLL;
        codec_zenoh_open_ack_encoded_t encoded = codec_zenoh_open_ack_encode(&actual);
        static const uint8_t _expected[] = { 0x01, 0x64 };
        if (encoded.len != sizeof _expected
            || memcmp(encoded.bytes, _expected, encoded.len) != 0) {
            fprintf(stderr,
                "FAIL: codec_zenoh_open_ack test_vector @SCXML L37: "
                "encode length=%zu (expected %zu)\n",
                encoded.len, sizeof _expected);
            ++failures;
        }
        sce_forge_cursor_t _cursor = sce_forge_cursor_init(_expected, sizeof _expected);
        codec_zenoh_open_ack_t decoded = {0};
        sce_forge_codec_status_t _st = codec_zenoh_open_ack_decode(&_cursor, &decoded);
        if (_st != SCE_FORGE_CODEC_OK) {
            fprintf(stderr,
                "FAIL: codec_zenoh_open_ack test_vector @SCXML L37: "
                "decode status=%d\n", (int)_st);
            ++failures;
        } else if (sce_forge_cursor_remaining(&_cursor) != 0) {
            fprintf(stderr,
                "FAIL: codec_zenoh_open_ack test_vector @SCXML L37: "
                "decode left %zu bytes unconsumed\n",
                sce_forge_cursor_remaining(&_cursor));
            ++failures;
        } else {
            if (decoded.lease != (uint64_t)0x1uLL) {
                fprintf(stderr,
                    "FAIL: codec_zenoh_open_ack test_vector @SCXML L37: "
                    "field `lease` mismatch\n");
                ++failures;
            }
            if (decoded.initial_sn != (uint64_t)0x64uLL) {
                fprintf(stderr,
                    "FAIL: codec_zenoh_open_ack test_vector @SCXML L37: "
                    "field `initial_sn` mismatch\n");
                ++failures;
            }
        }
    }
    {
        codec_zenoh_open_ack_t actual = {0};
        actual.lease = (uint64_t)0x7fuLL;
        actual.initial_sn = (uint64_t)0x1uLL;
        codec_zenoh_open_ack_encoded_t encoded = codec_zenoh_open_ack_encode(&actual);
        static const uint8_t _expected[] = { 0x7f, 0x01 };
        if (encoded.len != sizeof _expected
            || memcmp(encoded.bytes, _expected, encoded.len) != 0) {
            fprintf(stderr,
                "FAIL: codec_zenoh_open_ack test_vector @SCXML L41: "
                "encode length=%zu (expected %zu)\n",
                encoded.len, sizeof _expected);
            ++failures;
        }
        sce_forge_cursor_t _cursor = sce_forge_cursor_init(_expected, sizeof _expected);
        codec_zenoh_open_ack_t decoded = {0};
        sce_forge_codec_status_t _st = codec_zenoh_open_ack_decode(&_cursor, &decoded);
        if (_st != SCE_FORGE_CODEC_OK) {
            fprintf(stderr,
                "FAIL: codec_zenoh_open_ack test_vector @SCXML L41: "
                "decode status=%d\n", (int)_st);
            ++failures;
        } else if (sce_forge_cursor_remaining(&_cursor) != 0) {
            fprintf(stderr,
                "FAIL: codec_zenoh_open_ack test_vector @SCXML L41: "
                "decode left %zu bytes unconsumed\n",
                sce_forge_cursor_remaining(&_cursor));
            ++failures;
        } else {
            if (decoded.lease != (uint64_t)0x7fuLL) {
                fprintf(stderr,
                    "FAIL: codec_zenoh_open_ack test_vector @SCXML L41: "
                    "field `lease` mismatch\n");
                ++failures;
            }
            if (decoded.initial_sn != (uint64_t)0x1uLL) {
                fprintf(stderr,
                    "FAIL: codec_zenoh_open_ack test_vector @SCXML L41: "
                    "field `initial_sn` mismatch\n");
                ++failures;
            }
        }
    }
    {
        codec_zenoh_open_ack_t actual = {0};
        actual.lease = (uint64_t)0x80uLL;
        actual.initial_sn = (uint64_t)0xc8uLL;
        codec_zenoh_open_ack_encoded_t encoded = codec_zenoh_open_ack_encode(&actual);
        static const uint8_t _expected[] = { 0x80, 0x01, 0xc8, 0x01 };
        if (encoded.len != sizeof _expected
            || memcmp(encoded.bytes, _expected, encoded.len) != 0) {
            fprintf(stderr,
                "FAIL: codec_zenoh_open_ack test_vector @SCXML L45: "
                "encode length=%zu (expected %zu)\n",
                encoded.len, sizeof _expected);
            ++failures;
        }
        sce_forge_cursor_t _cursor = sce_forge_cursor_init(_expected, sizeof _expected);
        codec_zenoh_open_ack_t decoded = {0};
        sce_forge_codec_status_t _st = codec_zenoh_open_ack_decode(&_cursor, &decoded);
        if (_st != SCE_FORGE_CODEC_OK) {
            fprintf(stderr,
                "FAIL: codec_zenoh_open_ack test_vector @SCXML L45: "
                "decode status=%d\n", (int)_st);
            ++failures;
        } else if (sce_forge_cursor_remaining(&_cursor) != 0) {
            fprintf(stderr,
                "FAIL: codec_zenoh_open_ack test_vector @SCXML L45: "
                "decode left %zu bytes unconsumed\n",
                sce_forge_cursor_remaining(&_cursor));
            ++failures;
        } else {
            if (decoded.lease != (uint64_t)0x80uLL) {
                fprintf(stderr,
                    "FAIL: codec_zenoh_open_ack test_vector @SCXML L45: "
                    "field `lease` mismatch\n");
                ++failures;
            }
            if (decoded.initial_sn != (uint64_t)0xc8uLL) {
                fprintf(stderr,
                    "FAIL: codec_zenoh_open_ack test_vector @SCXML L45: "
                    "field `initial_sn` mismatch\n");
                ++failures;
            }
        }
    }
    return failures;
}

#endif  /* SCE_FORGE_CODEC_ZENOH_OPEN_ACK_TEST_H */
