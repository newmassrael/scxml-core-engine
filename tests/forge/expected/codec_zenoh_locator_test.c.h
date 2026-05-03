/* SCE Forge: Auto-generated codec test-vector sidecar (RFC §5.B B5-θ) */
/* Companion to codec_zenoh_locator.h — do not edit; regenerate from the source SCXML. */

#ifndef SCE_FORGE_CODEC_ZENOH_LOCATOR_TEST_H
#define SCE_FORGE_CODEC_ZENOH_LOCATOR_TEST_H

#include <stdio.h>
#include <string.h>
#include "codec_zenoh_locator.h"

static inline int test_vector_codec_zenoh_locator(void) {
    int failures = 0;
    {
        codec_zenoh_locator_t actual = {0};
        actual.locator_len = (uint64_t)0x0uLL;
        {
            static const char _str[] = "";
            /* sizeof _str includes the implicit NUL terminator; the
             * codec field stores raw UTF-8 bytes (no NUL) so subtract 1.
             * The decoder writes the byte length into the codec's
             * matching `<id>_len` companion (or directly to `len` for
             * the locator-style pair where the VLE-prefix field IS the
             * length sibling). */
            size_t _str_len = sizeof _str - 1;
            memcpy(actual.locator, _str, _str_len);
        }
        codec_zenoh_locator_encoded_t encoded = codec_zenoh_locator_encode(&actual);
        static const uint8_t _expected[] = { 0x00 };
        if (encoded.len != sizeof _expected
            || memcmp(encoded.bytes, _expected, encoded.len) != 0) {
            fprintf(stderr,
                "FAIL: codec_zenoh_locator test_vector @SCXML L38: "
                "encode length=%zu (expected %zu)\n",
                encoded.len, sizeof _expected);
            ++failures;
        }
        sce_forge_cursor_t _cursor = sce_forge_cursor_init(_expected, sizeof _expected);
        codec_zenoh_locator_t decoded = {0};
        sce_forge_codec_status_t _st = codec_zenoh_locator_decode(&_cursor, &decoded);
        if (_st != SCE_FORGE_CODEC_OK) {
            fprintf(stderr,
                "FAIL: codec_zenoh_locator test_vector @SCXML L38: "
                "decode status=%d\n", (int)_st);
            ++failures;
        } else if (sce_forge_cursor_remaining(&_cursor) != 0) {
            fprintf(stderr,
                "FAIL: codec_zenoh_locator test_vector @SCXML L38: "
                "decode left %zu bytes unconsumed\n",
                sce_forge_cursor_remaining(&_cursor));
            ++failures;
        } else {
            if (decoded.locator_len != (uint64_t)0x0uLL) {
                fprintf(stderr,
                    "FAIL: codec_zenoh_locator test_vector @SCXML L38: "
                    "field `locator_len` mismatch\n");
                ++failures;
            }
            {
                static const char _expected_str[] = "";
                size_t _expected_len = sizeof _expected_str - 1;
                /* The string codec uses the VLE-prefix length sibling
                 * (here `<id>_len`) as the byte count; the field
                 * `decoded.<id>_len` is set by the decoder. We compare
                 * raw bytes up to that authoritative length. */
                if (memcmp(decoded.locator,
                           _expected_str,
                           _expected_len) != 0) {
                    fprintf(stderr,
                        "FAIL: codec_zenoh_locator test_vector @SCXML L38: "
                        "field `locator` (string) mismatch\n");
                    ++failures;
                }
            }
        }
    }
    {
        codec_zenoh_locator_t actual = {0};
        actual.locator_len = (uint64_t)0x3uLL;
        {
            static const char _str[] = "abc";
            /* sizeof _str includes the implicit NUL terminator; the
             * codec field stores raw UTF-8 bytes (no NUL) so subtract 1.
             * The decoder writes the byte length into the codec's
             * matching `<id>_len` companion (or directly to `len` for
             * the locator-style pair where the VLE-prefix field IS the
             * length sibling). */
            size_t _str_len = sizeof _str - 1;
            memcpy(actual.locator, _str, _str_len);
        }
        codec_zenoh_locator_encoded_t encoded = codec_zenoh_locator_encode(&actual);
        static const uint8_t _expected[] = { 0x03, 0x61, 0x62, 0x63 };
        if (encoded.len != sizeof _expected
            || memcmp(encoded.bytes, _expected, encoded.len) != 0) {
            fprintf(stderr,
                "FAIL: codec_zenoh_locator test_vector @SCXML L42: "
                "encode length=%zu (expected %zu)\n",
                encoded.len, sizeof _expected);
            ++failures;
        }
        sce_forge_cursor_t _cursor = sce_forge_cursor_init(_expected, sizeof _expected);
        codec_zenoh_locator_t decoded = {0};
        sce_forge_codec_status_t _st = codec_zenoh_locator_decode(&_cursor, &decoded);
        if (_st != SCE_FORGE_CODEC_OK) {
            fprintf(stderr,
                "FAIL: codec_zenoh_locator test_vector @SCXML L42: "
                "decode status=%d\n", (int)_st);
            ++failures;
        } else if (sce_forge_cursor_remaining(&_cursor) != 0) {
            fprintf(stderr,
                "FAIL: codec_zenoh_locator test_vector @SCXML L42: "
                "decode left %zu bytes unconsumed\n",
                sce_forge_cursor_remaining(&_cursor));
            ++failures;
        } else {
            if (decoded.locator_len != (uint64_t)0x3uLL) {
                fprintf(stderr,
                    "FAIL: codec_zenoh_locator test_vector @SCXML L42: "
                    "field `locator_len` mismatch\n");
                ++failures;
            }
            {
                static const char _expected_str[] = "abc";
                size_t _expected_len = sizeof _expected_str - 1;
                /* The string codec uses the VLE-prefix length sibling
                 * (here `<id>_len`) as the byte count; the field
                 * `decoded.<id>_len` is set by the decoder. We compare
                 * raw bytes up to that authoritative length. */
                if (memcmp(decoded.locator,
                           _expected_str,
                           _expected_len) != 0) {
                    fprintf(stderr,
                        "FAIL: codec_zenoh_locator test_vector @SCXML L42: "
                        "field `locator` (string) mismatch\n");
                    ++failures;
                }
            }
        }
    }
    {
        codec_zenoh_locator_t actual = {0};
        actual.locator_len = (uint64_t)0x12uLL;
        {
            static const char _str[] = "tcp/127.0.0.1:7447";
            /* sizeof _str includes the implicit NUL terminator; the
             * codec field stores raw UTF-8 bytes (no NUL) so subtract 1.
             * The decoder writes the byte length into the codec's
             * matching `<id>_len` companion (or directly to `len` for
             * the locator-style pair where the VLE-prefix field IS the
             * length sibling). */
            size_t _str_len = sizeof _str - 1;
            memcpy(actual.locator, _str, _str_len);
        }
        codec_zenoh_locator_encoded_t encoded = codec_zenoh_locator_encode(&actual);
        static const uint8_t _expected[] = { 0x12, 0x74, 0x63, 0x70, 0x2f, 0x31, 0x32, 0x37, 0x2e, 0x30, 0x2e, 0x30, 0x2e, 0x31, 0x3a, 0x37, 0x34, 0x34, 0x37 };
        if (encoded.len != sizeof _expected
            || memcmp(encoded.bytes, _expected, encoded.len) != 0) {
            fprintf(stderr,
                "FAIL: codec_zenoh_locator test_vector @SCXML L46: "
                "encode length=%zu (expected %zu)\n",
                encoded.len, sizeof _expected);
            ++failures;
        }
        sce_forge_cursor_t _cursor = sce_forge_cursor_init(_expected, sizeof _expected);
        codec_zenoh_locator_t decoded = {0};
        sce_forge_codec_status_t _st = codec_zenoh_locator_decode(&_cursor, &decoded);
        if (_st != SCE_FORGE_CODEC_OK) {
            fprintf(stderr,
                "FAIL: codec_zenoh_locator test_vector @SCXML L46: "
                "decode status=%d\n", (int)_st);
            ++failures;
        } else if (sce_forge_cursor_remaining(&_cursor) != 0) {
            fprintf(stderr,
                "FAIL: codec_zenoh_locator test_vector @SCXML L46: "
                "decode left %zu bytes unconsumed\n",
                sce_forge_cursor_remaining(&_cursor));
            ++failures;
        } else {
            if (decoded.locator_len != (uint64_t)0x12uLL) {
                fprintf(stderr,
                    "FAIL: codec_zenoh_locator test_vector @SCXML L46: "
                    "field `locator_len` mismatch\n");
                ++failures;
            }
            {
                static const char _expected_str[] = "tcp/127.0.0.1:7447";
                size_t _expected_len = sizeof _expected_str - 1;
                /* The string codec uses the VLE-prefix length sibling
                 * (here `<id>_len`) as the byte count; the field
                 * `decoded.<id>_len` is set by the decoder. We compare
                 * raw bytes up to that authoritative length. */
                if (memcmp(decoded.locator,
                           _expected_str,
                           _expected_len) != 0) {
                    fprintf(stderr,
                        "FAIL: codec_zenoh_locator test_vector @SCXML L46: "
                        "field `locator` (string) mismatch\n");
                    ++failures;
                }
            }
        }
    }
    {
        codec_zenoh_locator_t actual = {0};
        actual.locator_len = (uint64_t)0x6uLL;
        {
            static const char _str[] = "héllo";
            /* sizeof _str includes the implicit NUL terminator; the
             * codec field stores raw UTF-8 bytes (no NUL) so subtract 1.
             * The decoder writes the byte length into the codec's
             * matching `<id>_len` companion (or directly to `len` for
             * the locator-style pair where the VLE-prefix field IS the
             * length sibling). */
            size_t _str_len = sizeof _str - 1;
            memcpy(actual.locator, _str, _str_len);
        }
        codec_zenoh_locator_encoded_t encoded = codec_zenoh_locator_encode(&actual);
        static const uint8_t _expected[] = { 0x06, 0x68, 0xc3, 0xa9, 0x6c, 0x6c, 0x6f };
        if (encoded.len != sizeof _expected
            || memcmp(encoded.bytes, _expected, encoded.len) != 0) {
            fprintf(stderr,
                "FAIL: codec_zenoh_locator test_vector @SCXML L50: "
                "encode length=%zu (expected %zu)\n",
                encoded.len, sizeof _expected);
            ++failures;
        }
        sce_forge_cursor_t _cursor = sce_forge_cursor_init(_expected, sizeof _expected);
        codec_zenoh_locator_t decoded = {0};
        sce_forge_codec_status_t _st = codec_zenoh_locator_decode(&_cursor, &decoded);
        if (_st != SCE_FORGE_CODEC_OK) {
            fprintf(stderr,
                "FAIL: codec_zenoh_locator test_vector @SCXML L50: "
                "decode status=%d\n", (int)_st);
            ++failures;
        } else if (sce_forge_cursor_remaining(&_cursor) != 0) {
            fprintf(stderr,
                "FAIL: codec_zenoh_locator test_vector @SCXML L50: "
                "decode left %zu bytes unconsumed\n",
                sce_forge_cursor_remaining(&_cursor));
            ++failures;
        } else {
            if (decoded.locator_len != (uint64_t)0x6uLL) {
                fprintf(stderr,
                    "FAIL: codec_zenoh_locator test_vector @SCXML L50: "
                    "field `locator_len` mismatch\n");
                ++failures;
            }
            {
                static const char _expected_str[] = "héllo";
                size_t _expected_len = sizeof _expected_str - 1;
                /* The string codec uses the VLE-prefix length sibling
                 * (here `<id>_len`) as the byte count; the field
                 * `decoded.<id>_len` is set by the decoder. We compare
                 * raw bytes up to that authoritative length. */
                if (memcmp(decoded.locator,
                           _expected_str,
                           _expected_len) != 0) {
                    fprintf(stderr,
                        "FAIL: codec_zenoh_locator test_vector @SCXML L50: "
                        "field `locator` (string) mismatch\n");
                    ++failures;
                }
            }
        }
    }
    return failures;
}

#endif  /* SCE_FORGE_CODEC_ZENOH_LOCATOR_TEST_H */
