/* SCE Forge: Auto-generated codec test-vector sidecar (RFC §synth-5-B) */
/* Companion to codec_fixed_after_lengthref.h — do not edit; regenerate from the source SCXML. */

#ifndef SCE_FORGE_CODEC_FIXED_AFTER_LENGTHREF_TEST_H
#define SCE_FORGE_CODEC_FIXED_AFTER_LENGTHREF_TEST_H

#include <stdio.h>
#include <string.h>
#include "codec_fixed_after_lengthref.h"

static inline int test_vector_codec_fixed_after_lengthref(void) {
    int failures = 0;
    {
        codec_fixed_after_lengthref_t actual = {0};
        actual.header = (uint8_t)0xaau;
        actual.payload_len = (uint16_t)0x3u;
        {
            static const uint8_t _bytes[] = { 0xde, 0xad, 0xbe };
            memcpy(actual.payload, _bytes, sizeof _bytes);
            actual.payload_len = sizeof _bytes;
        }
        actual.crc32 = (uint32_t)0x11223344u;
        uint8_t _encoded_buf[CODEC_FIXED_AFTER_LENGTHREF_MAX_BYTES];
        size_t _encoded_len = 0;
        sce_forge_codec_status_t _enc_st = codec_fixed_after_lengthref_encode_to_buf(&actual, _encoded_buf, sizeof(_encoded_buf), &_encoded_len);
        (void)_enc_st;
        static const uint8_t _expected[] = { 0xaa, 0x03, 0x00, 0xde, 0xad, 0xbe, 0x44, 0x33, 0x22, 0x11 };
        if (_encoded_len != sizeof _expected
            || memcmp(_encoded_buf, _expected, _encoded_len) != 0) {
            fprintf(stderr,
                "FAIL: codec_fixed_after_lengthref test_vector @SCXML L30: "
                "encode length=%zu (expected %zu)\n",
                _encoded_len, sizeof _expected);
            ++failures;
        }
        sce_forge_cursor_t _cursor = sce_forge_cursor_init(_expected, sizeof _expected);
        codec_fixed_after_lengthref_t decoded = {0};
        sce_forge_codec_status_t _st = codec_fixed_after_lengthref_decode(&_cursor, &decoded);
        if (_st != SCE_FORGE_CODEC_OK) {
            fprintf(stderr,
                "FAIL: codec_fixed_after_lengthref test_vector @SCXML L30: "
                "decode status=%d\n", (int)_st);
            ++failures;
        } else if (sce_forge_cursor_remaining(&_cursor) != 0) {
            fprintf(stderr,
                "FAIL: codec_fixed_after_lengthref test_vector @SCXML L30: "
                "decode left %zu bytes unconsumed\n",
                sce_forge_cursor_remaining(&_cursor));
            ++failures;
        } else {
            if (decoded.header != (uint8_t)0xaau) {
                fprintf(stderr,
                    "FAIL: codec_fixed_after_lengthref test_vector @SCXML L30: "
                    "field `header` mismatch\n");
                ++failures;
            }
            if (decoded.payload_len != (uint16_t)0x3u) {
                fprintf(stderr,
                    "FAIL: codec_fixed_after_lengthref test_vector @SCXML L30: "
                    "field `payload_len` mismatch\n");
                ++failures;
            }
            {
                static const uint8_t _expected_field[] = { 0xde, 0xad, 0xbe };
                if (decoded.payload_len != sizeof _expected_field
                    || memcmp(decoded.payload,
                              _expected_field,
                              decoded.payload_len) != 0) {
                    fprintf(stderr,
                        "FAIL: codec_fixed_after_lengthref test_vector @SCXML L30: "
                        "field `payload` (bytes) mismatch\n");
                    ++failures;
                }
            }
            if (decoded.crc32 != (uint32_t)0x11223344u) {
                fprintf(stderr,
                    "FAIL: codec_fixed_after_lengthref test_vector @SCXML L30: "
                    "field `crc32` mismatch\n");
                ++failures;
            }
        }
    }
    {
        codec_fixed_after_lengthref_t actual = {0};
        actual.header = (uint8_t)0x1u;
        actual.payload_len = (uint16_t)0x0u;
        {
            actual.payload_len = 0;
        }
        actual.crc32 = (uint32_t)0xcafebabeu;
        uint8_t _encoded_buf[CODEC_FIXED_AFTER_LENGTHREF_MAX_BYTES];
        size_t _encoded_len = 0;
        sce_forge_codec_status_t _enc_st = codec_fixed_after_lengthref_encode_to_buf(&actual, _encoded_buf, sizeof(_encoded_buf), &_encoded_len);
        (void)_enc_st;
        static const uint8_t _expected[] = { 0x01, 0x00, 0x00, 0xbe, 0xba, 0xfe, 0xca };
        if (_encoded_len != sizeof _expected
            || memcmp(_encoded_buf, _expected, _encoded_len) != 0) {
            fprintf(stderr,
                "FAIL: codec_fixed_after_lengthref test_vector @SCXML L37: "
                "encode length=%zu (expected %zu)\n",
                _encoded_len, sizeof _expected);
            ++failures;
        }
        sce_forge_cursor_t _cursor = sce_forge_cursor_init(_expected, sizeof _expected);
        codec_fixed_after_lengthref_t decoded = {0};
        sce_forge_codec_status_t _st = codec_fixed_after_lengthref_decode(&_cursor, &decoded);
        if (_st != SCE_FORGE_CODEC_OK) {
            fprintf(stderr,
                "FAIL: codec_fixed_after_lengthref test_vector @SCXML L37: "
                "decode status=%d\n", (int)_st);
            ++failures;
        } else if (sce_forge_cursor_remaining(&_cursor) != 0) {
            fprintf(stderr,
                "FAIL: codec_fixed_after_lengthref test_vector @SCXML L37: "
                "decode left %zu bytes unconsumed\n",
                sce_forge_cursor_remaining(&_cursor));
            ++failures;
        } else {
            if (decoded.header != (uint8_t)0x1u) {
                fprintf(stderr,
                    "FAIL: codec_fixed_after_lengthref test_vector @SCXML L37: "
                    "field `header` mismatch\n");
                ++failures;
            }
            if (decoded.payload_len != (uint16_t)0x0u) {
                fprintf(stderr,
                    "FAIL: codec_fixed_after_lengthref test_vector @SCXML L37: "
                    "field `payload_len` mismatch\n");
                ++failures;
            }
            {
                if (decoded.payload_len != 0) {
                    fprintf(stderr,
                        "FAIL: codec_fixed_after_lengthref test_vector @SCXML L37: "
                        "field `payload` expected empty bytes, got len=%zu\n",
                        /* The length companion is `size_t` for a tail field but
                         * the (possibly narrower) wire length-field for a
                         * length-ref payload — cast so `%zu` is always valid. */
                        (size_t)decoded.payload_len);
                    ++failures;
                }
            }
            if (decoded.crc32 != (uint32_t)0xcafebabeu) {
                fprintf(stderr,
                    "FAIL: codec_fixed_after_lengthref test_vector @SCXML L37: "
                    "field `crc32` mismatch\n");
                ++failures;
            }
        }
    }
    {
        codec_fixed_after_lengthref_t actual = {0};
        actual.header = (uint8_t)0xffu;
        actual.payload_len = (uint16_t)0x5u;
        {
            static const uint8_t _bytes[] = { 0x01, 0x02, 0x03, 0x04, 0x05 };
            memcpy(actual.payload, _bytes, sizeof _bytes);
            actual.payload_len = sizeof _bytes;
        }
        actual.crc32 = (uint32_t)0x1u;
        uint8_t _encoded_buf[CODEC_FIXED_AFTER_LENGTHREF_MAX_BYTES];
        size_t _encoded_len = 0;
        sce_forge_codec_status_t _enc_st = codec_fixed_after_lengthref_encode_to_buf(&actual, _encoded_buf, sizeof(_encoded_buf), &_encoded_len);
        (void)_enc_st;
        static const uint8_t _expected[] = { 0xff, 0x05, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x01, 0x00, 0x00, 0x00 };
        if (_encoded_len != sizeof _expected
            || memcmp(_encoded_buf, _expected, _encoded_len) != 0) {
            fprintf(stderr,
                "FAIL: codec_fixed_after_lengthref test_vector @SCXML L44: "
                "encode length=%zu (expected %zu)\n",
                _encoded_len, sizeof _expected);
            ++failures;
        }
        sce_forge_cursor_t _cursor = sce_forge_cursor_init(_expected, sizeof _expected);
        codec_fixed_after_lengthref_t decoded = {0};
        sce_forge_codec_status_t _st = codec_fixed_after_lengthref_decode(&_cursor, &decoded);
        if (_st != SCE_FORGE_CODEC_OK) {
            fprintf(stderr,
                "FAIL: codec_fixed_after_lengthref test_vector @SCXML L44: "
                "decode status=%d\n", (int)_st);
            ++failures;
        } else if (sce_forge_cursor_remaining(&_cursor) != 0) {
            fprintf(stderr,
                "FAIL: codec_fixed_after_lengthref test_vector @SCXML L44: "
                "decode left %zu bytes unconsumed\n",
                sce_forge_cursor_remaining(&_cursor));
            ++failures;
        } else {
            if (decoded.header != (uint8_t)0xffu) {
                fprintf(stderr,
                    "FAIL: codec_fixed_after_lengthref test_vector @SCXML L44: "
                    "field `header` mismatch\n");
                ++failures;
            }
            if (decoded.payload_len != (uint16_t)0x5u) {
                fprintf(stderr,
                    "FAIL: codec_fixed_after_lengthref test_vector @SCXML L44: "
                    "field `payload_len` mismatch\n");
                ++failures;
            }
            {
                static const uint8_t _expected_field[] = { 0x01, 0x02, 0x03, 0x04, 0x05 };
                if (decoded.payload_len != sizeof _expected_field
                    || memcmp(decoded.payload,
                              _expected_field,
                              decoded.payload_len) != 0) {
                    fprintf(stderr,
                        "FAIL: codec_fixed_after_lengthref test_vector @SCXML L44: "
                        "field `payload` (bytes) mismatch\n");
                    ++failures;
                }
            }
            if (decoded.crc32 != (uint32_t)0x1u) {
                fprintf(stderr,
                    "FAIL: codec_fixed_after_lengthref test_vector @SCXML L44: "
                    "field `crc32` mismatch\n");
                ++failures;
            }
        }
    }
    return failures;
}

#endif  /* SCE_FORGE_CODEC_FIXED_AFTER_LENGTHREF_TEST_H */
