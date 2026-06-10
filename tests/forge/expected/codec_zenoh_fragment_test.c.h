/* SCE Forge: Auto-generated codec test-vector sidecar (RFC §synth-5-B) */
/* Companion to codec_zenoh_fragment.h — do not edit; regenerate from the source SCXML. */

#ifndef SCE_FORGE_CODEC_ZENOH_FRAGMENT_TEST_H
#define SCE_FORGE_CODEC_ZENOH_FRAGMENT_TEST_H

#include <stdio.h>
#include <string.h>
#include "codec_zenoh_fragment.h"

static inline int test_vector_codec_zenoh_fragment(void) {
    int failures = 0;
    {
        codec_zenoh_fragment_t actual = {0};
        actual.sn = (uint64_t)0x0uLL;
        {
            actual.payload_len = 0;
        }
        uint8_t _encoded_buf[CODEC_ZENOH_FRAGMENT_MAX_BYTES];
        size_t _encoded_len = 0;
        sce_forge_codec_status_t _enc_st = codec_zenoh_fragment_encode_to_buf(&actual, _encoded_buf, sizeof(_encoded_buf), &_encoded_len);
        (void)_enc_st;
        static const uint8_t _expected[] = { 0x00 };
        if (_encoded_len != sizeof _expected
            || memcmp(_encoded_buf, _expected, _encoded_len) != 0) {
            fprintf(stderr,
                "FAIL: codec_zenoh_fragment test_vector @SCXML L35: "
                "encode length=%zu (expected %zu)\n",
                _encoded_len, sizeof _expected);
            ++failures;
        }
        sce_forge_cursor_t _cursor = sce_forge_cursor_init(_expected, sizeof _expected);
        codec_zenoh_fragment_t decoded = {0};
        sce_forge_codec_status_t _st = codec_zenoh_fragment_decode(&_cursor, &decoded);
        if (_st != SCE_FORGE_CODEC_OK) {
            fprintf(stderr,
                "FAIL: codec_zenoh_fragment test_vector @SCXML L35: "
                "decode status=%d\n", (int)_st);
            ++failures;
        } else if (sce_forge_cursor_remaining(&_cursor) != 0) {
            fprintf(stderr,
                "FAIL: codec_zenoh_fragment test_vector @SCXML L35: "
                "decode left %zu bytes unconsumed\n",
                sce_forge_cursor_remaining(&_cursor));
            ++failures;
        } else {
            if (decoded.sn != (uint64_t)0x0uLL) {
                fprintf(stderr,
                    "FAIL: codec_zenoh_fragment test_vector @SCXML L35: "
                    "field `sn` mismatch\n");
                ++failures;
            }
            {
                if (decoded.payload_len != 0) {
                    fprintf(stderr,
                        "FAIL: codec_zenoh_fragment test_vector @SCXML L35: "
                        "field `payload` expected empty bytes, got len=%zu\n",
                        decoded.payload_len);
                    ++failures;
                }
            }
        }
    }
    {
        codec_zenoh_fragment_t actual = {0};
        actual.sn = (uint64_t)0x1uLL;
        {
            static const uint8_t _bytes[] = { 0xca, 0xfe };
            memcpy(actual.payload, _bytes, sizeof _bytes);
            actual.payload_len = sizeof _bytes;
        }
        uint8_t _encoded_buf[CODEC_ZENOH_FRAGMENT_MAX_BYTES];
        size_t _encoded_len = 0;
        sce_forge_codec_status_t _enc_st = codec_zenoh_fragment_encode_to_buf(&actual, _encoded_buf, sizeof(_encoded_buf), &_encoded_len);
        (void)_enc_st;
        static const uint8_t _expected[] = { 0x01, 0xca, 0xfe };
        if (_encoded_len != sizeof _expected
            || memcmp(_encoded_buf, _expected, _encoded_len) != 0) {
            fprintf(stderr,
                "FAIL: codec_zenoh_fragment test_vector @SCXML L39: "
                "encode length=%zu (expected %zu)\n",
                _encoded_len, sizeof _expected);
            ++failures;
        }
        sce_forge_cursor_t _cursor = sce_forge_cursor_init(_expected, sizeof _expected);
        codec_zenoh_fragment_t decoded = {0};
        sce_forge_codec_status_t _st = codec_zenoh_fragment_decode(&_cursor, &decoded);
        if (_st != SCE_FORGE_CODEC_OK) {
            fprintf(stderr,
                "FAIL: codec_zenoh_fragment test_vector @SCXML L39: "
                "decode status=%d\n", (int)_st);
            ++failures;
        } else if (sce_forge_cursor_remaining(&_cursor) != 0) {
            fprintf(stderr,
                "FAIL: codec_zenoh_fragment test_vector @SCXML L39: "
                "decode left %zu bytes unconsumed\n",
                sce_forge_cursor_remaining(&_cursor));
            ++failures;
        } else {
            if (decoded.sn != (uint64_t)0x1uLL) {
                fprintf(stderr,
                    "FAIL: codec_zenoh_fragment test_vector @SCXML L39: "
                    "field `sn` mismatch\n");
                ++failures;
            }
            {
                static const uint8_t _expected_field[] = { 0xca, 0xfe };
                if (decoded.payload_len != sizeof _expected_field
                    || memcmp(decoded.payload,
                              _expected_field,
                              decoded.payload_len) != 0) {
                    fprintf(stderr,
                        "FAIL: codec_zenoh_fragment test_vector @SCXML L39: "
                        "field `payload` (bytes) mismatch\n");
                    ++failures;
                }
            }
        }
    }
    {
        codec_zenoh_fragment_t actual = {0};
        actual.sn = (uint64_t)0x7fuLL;
        {
            static const uint8_t _bytes[] = { 0xaa, 0xbb, 0xcc };
            memcpy(actual.payload, _bytes, sizeof _bytes);
            actual.payload_len = sizeof _bytes;
        }
        uint8_t _encoded_buf[CODEC_ZENOH_FRAGMENT_MAX_BYTES];
        size_t _encoded_len = 0;
        sce_forge_codec_status_t _enc_st = codec_zenoh_fragment_encode_to_buf(&actual, _encoded_buf, sizeof(_encoded_buf), &_encoded_len);
        (void)_enc_st;
        static const uint8_t _expected[] = { 0x7f, 0xaa, 0xbb, 0xcc };
        if (_encoded_len != sizeof _expected
            || memcmp(_encoded_buf, _expected, _encoded_len) != 0) {
            fprintf(stderr,
                "FAIL: codec_zenoh_fragment test_vector @SCXML L43: "
                "encode length=%zu (expected %zu)\n",
                _encoded_len, sizeof _expected);
            ++failures;
        }
        sce_forge_cursor_t _cursor = sce_forge_cursor_init(_expected, sizeof _expected);
        codec_zenoh_fragment_t decoded = {0};
        sce_forge_codec_status_t _st = codec_zenoh_fragment_decode(&_cursor, &decoded);
        if (_st != SCE_FORGE_CODEC_OK) {
            fprintf(stderr,
                "FAIL: codec_zenoh_fragment test_vector @SCXML L43: "
                "decode status=%d\n", (int)_st);
            ++failures;
        } else if (sce_forge_cursor_remaining(&_cursor) != 0) {
            fprintf(stderr,
                "FAIL: codec_zenoh_fragment test_vector @SCXML L43: "
                "decode left %zu bytes unconsumed\n",
                sce_forge_cursor_remaining(&_cursor));
            ++failures;
        } else {
            if (decoded.sn != (uint64_t)0x7fuLL) {
                fprintf(stderr,
                    "FAIL: codec_zenoh_fragment test_vector @SCXML L43: "
                    "field `sn` mismatch\n");
                ++failures;
            }
            {
                static const uint8_t _expected_field[] = { 0xaa, 0xbb, 0xcc };
                if (decoded.payload_len != sizeof _expected_field
                    || memcmp(decoded.payload,
                              _expected_field,
                              decoded.payload_len) != 0) {
                    fprintf(stderr,
                        "FAIL: codec_zenoh_fragment test_vector @SCXML L43: "
                        "field `payload` (bytes) mismatch\n");
                    ++failures;
                }
            }
        }
    }
    {
        codec_zenoh_fragment_t actual = {0};
        actual.sn = (uint64_t)0x80uLL;
        {
            static const uint8_t _bytes[] = { 0xde, 0xad };
            memcpy(actual.payload, _bytes, sizeof _bytes);
            actual.payload_len = sizeof _bytes;
        }
        uint8_t _encoded_buf[CODEC_ZENOH_FRAGMENT_MAX_BYTES];
        size_t _encoded_len = 0;
        sce_forge_codec_status_t _enc_st = codec_zenoh_fragment_encode_to_buf(&actual, _encoded_buf, sizeof(_encoded_buf), &_encoded_len);
        (void)_enc_st;
        static const uint8_t _expected[] = { 0x80, 0x01, 0xde, 0xad };
        if (_encoded_len != sizeof _expected
            || memcmp(_encoded_buf, _expected, _encoded_len) != 0) {
            fprintf(stderr,
                "FAIL: codec_zenoh_fragment test_vector @SCXML L47: "
                "encode length=%zu (expected %zu)\n",
                _encoded_len, sizeof _expected);
            ++failures;
        }
        sce_forge_cursor_t _cursor = sce_forge_cursor_init(_expected, sizeof _expected);
        codec_zenoh_fragment_t decoded = {0};
        sce_forge_codec_status_t _st = codec_zenoh_fragment_decode(&_cursor, &decoded);
        if (_st != SCE_FORGE_CODEC_OK) {
            fprintf(stderr,
                "FAIL: codec_zenoh_fragment test_vector @SCXML L47: "
                "decode status=%d\n", (int)_st);
            ++failures;
        } else if (sce_forge_cursor_remaining(&_cursor) != 0) {
            fprintf(stderr,
                "FAIL: codec_zenoh_fragment test_vector @SCXML L47: "
                "decode left %zu bytes unconsumed\n",
                sce_forge_cursor_remaining(&_cursor));
            ++failures;
        } else {
            if (decoded.sn != (uint64_t)0x80uLL) {
                fprintf(stderr,
                    "FAIL: codec_zenoh_fragment test_vector @SCXML L47: "
                    "field `sn` mismatch\n");
                ++failures;
            }
            {
                static const uint8_t _expected_field[] = { 0xde, 0xad };
                if (decoded.payload_len != sizeof _expected_field
                    || memcmp(decoded.payload,
                              _expected_field,
                              decoded.payload_len) != 0) {
                    fprintf(stderr,
                        "FAIL: codec_zenoh_fragment test_vector @SCXML L47: "
                        "field `payload` (bytes) mismatch\n");
                    ++failures;
                }
            }
        }
    }
    return failures;
}

#endif  /* SCE_FORGE_CODEC_ZENOH_FRAGMENT_TEST_H */
