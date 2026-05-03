/* SCE Forge: Auto-generated codec test-vector sidecar (RFC §5.B B5-θ) */
/* Companion to codec_zenoh_close.h — do not edit; regenerate from the source SCXML. */

#ifndef SCE_FORGE_CODEC_ZENOH_CLOSE_TEST_H
#define SCE_FORGE_CODEC_ZENOH_CLOSE_TEST_H

#include <stdio.h>
#include <string.h>
#include "codec_zenoh_close.h"

static inline int test_vector_codec_zenoh_close(void) {
    int failures = 0;
    {
        codec_zenoh_close_t actual = {0};
        actual.reason = (uint8_t)0x0u;
        codec_zenoh_close_encoded_t encoded = codec_zenoh_close_encode(&actual);
        static const uint8_t _expected[] = { 0x00 };
        if (encoded.len != sizeof _expected
            || memcmp(encoded.bytes, _expected, encoded.len) != 0) {
            fprintf(stderr,
                "FAIL: codec_zenoh_close test_vector @SCXML L27: "
                "encode length=%zu (expected %zu)\n",
                encoded.len, sizeof _expected);
            ++failures;
        }
        sce_forge_cursor_t _cursor = sce_forge_cursor_init(_expected, sizeof _expected);
        codec_zenoh_close_t decoded = {0};
        sce_forge_codec_status_t _st = codec_zenoh_close_decode(&_cursor, &decoded);
        if (_st != SCE_FORGE_CODEC_OK) {
            fprintf(stderr,
                "FAIL: codec_zenoh_close test_vector @SCXML L27: "
                "decode status=%d\n", (int)_st);
            ++failures;
        } else if (sce_forge_cursor_remaining(&_cursor) != 0) {
            fprintf(stderr,
                "FAIL: codec_zenoh_close test_vector @SCXML L27: "
                "decode left %zu bytes unconsumed\n",
                sce_forge_cursor_remaining(&_cursor));
            ++failures;
        } else {
            if (decoded.reason != (uint8_t)0x0u) {
                fprintf(stderr,
                    "FAIL: codec_zenoh_close test_vector @SCXML L27: "
                    "field `reason` mismatch\n");
                ++failures;
            }
        }
    }
    {
        codec_zenoh_close_t actual = {0};
        actual.reason = (uint8_t)0x1u;
        codec_zenoh_close_encoded_t encoded = codec_zenoh_close_encode(&actual);
        static const uint8_t _expected[] = { 0x01 };
        if (encoded.len != sizeof _expected
            || memcmp(encoded.bytes, _expected, encoded.len) != 0) {
            fprintf(stderr,
                "FAIL: codec_zenoh_close test_vector @SCXML L30: "
                "encode length=%zu (expected %zu)\n",
                encoded.len, sizeof _expected);
            ++failures;
        }
        sce_forge_cursor_t _cursor = sce_forge_cursor_init(_expected, sizeof _expected);
        codec_zenoh_close_t decoded = {0};
        sce_forge_codec_status_t _st = codec_zenoh_close_decode(&_cursor, &decoded);
        if (_st != SCE_FORGE_CODEC_OK) {
            fprintf(stderr,
                "FAIL: codec_zenoh_close test_vector @SCXML L30: "
                "decode status=%d\n", (int)_st);
            ++failures;
        } else if (sce_forge_cursor_remaining(&_cursor) != 0) {
            fprintf(stderr,
                "FAIL: codec_zenoh_close test_vector @SCXML L30: "
                "decode left %zu bytes unconsumed\n",
                sce_forge_cursor_remaining(&_cursor));
            ++failures;
        } else {
            if (decoded.reason != (uint8_t)0x1u) {
                fprintf(stderr,
                    "FAIL: codec_zenoh_close test_vector @SCXML L30: "
                    "field `reason` mismatch\n");
                ++failures;
            }
        }
    }
    {
        codec_zenoh_close_t actual = {0};
        actual.reason = (uint8_t)0x2u;
        codec_zenoh_close_encoded_t encoded = codec_zenoh_close_encode(&actual);
        static const uint8_t _expected[] = { 0x02 };
        if (encoded.len != sizeof _expected
            || memcmp(encoded.bytes, _expected, encoded.len) != 0) {
            fprintf(stderr,
                "FAIL: codec_zenoh_close test_vector @SCXML L33: "
                "encode length=%zu (expected %zu)\n",
                encoded.len, sizeof _expected);
            ++failures;
        }
        sce_forge_cursor_t _cursor = sce_forge_cursor_init(_expected, sizeof _expected);
        codec_zenoh_close_t decoded = {0};
        sce_forge_codec_status_t _st = codec_zenoh_close_decode(&_cursor, &decoded);
        if (_st != SCE_FORGE_CODEC_OK) {
            fprintf(stderr,
                "FAIL: codec_zenoh_close test_vector @SCXML L33: "
                "decode status=%d\n", (int)_st);
            ++failures;
        } else if (sce_forge_cursor_remaining(&_cursor) != 0) {
            fprintf(stderr,
                "FAIL: codec_zenoh_close test_vector @SCXML L33: "
                "decode left %zu bytes unconsumed\n",
                sce_forge_cursor_remaining(&_cursor));
            ++failures;
        } else {
            if (decoded.reason != (uint8_t)0x2u) {
                fprintf(stderr,
                    "FAIL: codec_zenoh_close test_vector @SCXML L33: "
                    "field `reason` mismatch\n");
                ++failures;
            }
        }
    }
    {
        codec_zenoh_close_t actual = {0};
        actual.reason = (uint8_t)0xffu;
        codec_zenoh_close_encoded_t encoded = codec_zenoh_close_encode(&actual);
        static const uint8_t _expected[] = { 0xff };
        if (encoded.len != sizeof _expected
            || memcmp(encoded.bytes, _expected, encoded.len) != 0) {
            fprintf(stderr,
                "FAIL: codec_zenoh_close test_vector @SCXML L36: "
                "encode length=%zu (expected %zu)\n",
                encoded.len, sizeof _expected);
            ++failures;
        }
        sce_forge_cursor_t _cursor = sce_forge_cursor_init(_expected, sizeof _expected);
        codec_zenoh_close_t decoded = {0};
        sce_forge_codec_status_t _st = codec_zenoh_close_decode(&_cursor, &decoded);
        if (_st != SCE_FORGE_CODEC_OK) {
            fprintf(stderr,
                "FAIL: codec_zenoh_close test_vector @SCXML L36: "
                "decode status=%d\n", (int)_st);
            ++failures;
        } else if (sce_forge_cursor_remaining(&_cursor) != 0) {
            fprintf(stderr,
                "FAIL: codec_zenoh_close test_vector @SCXML L36: "
                "decode left %zu bytes unconsumed\n",
                sce_forge_cursor_remaining(&_cursor));
            ++failures;
        } else {
            if (decoded.reason != (uint8_t)0xffu) {
                fprintf(stderr,
                    "FAIL: codec_zenoh_close test_vector @SCXML L36: "
                    "field `reason` mismatch\n");
                ++failures;
            }
        }
    }
    return failures;
}

#endif  /* SCE_FORGE_CODEC_ZENOH_CLOSE_TEST_H */
