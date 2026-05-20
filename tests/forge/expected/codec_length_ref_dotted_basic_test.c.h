/* SCE Forge: Auto-generated codec test-vector sidecar (RFC §5.B B5-θ) */
/* Companion to codec_length_ref_dotted_basic.h — do not edit; regenerate from the source SCXML. */

#ifndef SCE_FORGE_CODEC_LENGTH_REF_DOTTED_BASIC_TEST_H
#define SCE_FORGE_CODEC_LENGTH_REF_DOTTED_BASIC_TEST_H

#include <stdio.h>
#include <string.h>
#include "codec_length_ref_dotted_basic.h"

static inline int test_vector_codec_length_ref_dotted_basic(void) {
    int failures = 0;
    {
        codec_length_ref_dotted_basic_t actual = {0};
        actual.carrier = (uint8_t)0x0u;
        {
            actual.payload_len = 0;
        }
        uint8_t _encoded_buf[CODEC_LENGTH_REF_DOTTED_BASIC_MAX_BYTES];
        size_t _encoded_len = 0;
        sce_forge_codec_status_t _enc_st = codec_length_ref_dotted_basic_encode_to_buf(&actual, _encoded_buf, sizeof(_encoded_buf), &_encoded_len);
        (void)_enc_st;
        static const uint8_t _expected[] = { 0x00 };
        if (_encoded_len != sizeof _expected
            || memcmp(_encoded_buf, _expected, _encoded_len) != 0) {
            fprintf(stderr,
                "FAIL: codec_length_ref_dotted_basic test_vector @SCXML L41: "
                "encode length=%zu (expected %zu)\n",
                _encoded_len, sizeof _expected);
            ++failures;
        }
        sce_forge_cursor_t _cursor = sce_forge_cursor_init(_expected, sizeof _expected);
        codec_length_ref_dotted_basic_t decoded = {0};
        sce_forge_codec_status_t _st = codec_length_ref_dotted_basic_decode(&_cursor, &decoded);
        if (_st != SCE_FORGE_CODEC_OK) {
            fprintf(stderr,
                "FAIL: codec_length_ref_dotted_basic test_vector @SCXML L41: "
                "decode status=%d\n", (int)_st);
            ++failures;
        } else if (sce_forge_cursor_remaining(&_cursor) != 0) {
            fprintf(stderr,
                "FAIL: codec_length_ref_dotted_basic test_vector @SCXML L41: "
                "decode left %zu bytes unconsumed\n",
                sce_forge_cursor_remaining(&_cursor));
            ++failures;
        } else {
            if (decoded.carrier != (uint8_t)0x0u) {
                fprintf(stderr,
                    "FAIL: codec_length_ref_dotted_basic test_vector @SCXML L41: "
                    "field `carrier` mismatch\n");
                ++failures;
            }
            {
                if (decoded.payload_len != 0) {
                    fprintf(stderr,
                        "FAIL: codec_length_ref_dotted_basic test_vector @SCXML L41: "
                        "field `payload` expected empty bytes, got len=%zu\n",
                        decoded.payload_len);
                    ++failures;
                }
            }
        }
    }
    {
        codec_length_ref_dotted_basic_t actual = {0};
        actual.carrier = (uint8_t)0x21u;
        {
            static const uint8_t _bytes[] = { 0xaa, 0xbb };
            memcpy(actual.payload, _bytes, sizeof _bytes);
            actual.payload_len = sizeof _bytes;
        }
        uint8_t _encoded_buf[CODEC_LENGTH_REF_DOTTED_BASIC_MAX_BYTES];
        size_t _encoded_len = 0;
        sce_forge_codec_status_t _enc_st = codec_length_ref_dotted_basic_encode_to_buf(&actual, _encoded_buf, sizeof(_encoded_buf), &_encoded_len);
        (void)_enc_st;
        static const uint8_t _expected[] = { 0x21, 0xaa, 0xbb };
        if (_encoded_len != sizeof _expected
            || memcmp(_encoded_buf, _expected, _encoded_len) != 0) {
            fprintf(stderr,
                "FAIL: codec_length_ref_dotted_basic test_vector @SCXML L45: "
                "encode length=%zu (expected %zu)\n",
                _encoded_len, sizeof _expected);
            ++failures;
        }
        sce_forge_cursor_t _cursor = sce_forge_cursor_init(_expected, sizeof _expected);
        codec_length_ref_dotted_basic_t decoded = {0};
        sce_forge_codec_status_t _st = codec_length_ref_dotted_basic_decode(&_cursor, &decoded);
        if (_st != SCE_FORGE_CODEC_OK) {
            fprintf(stderr,
                "FAIL: codec_length_ref_dotted_basic test_vector @SCXML L45: "
                "decode status=%d\n", (int)_st);
            ++failures;
        } else if (sce_forge_cursor_remaining(&_cursor) != 0) {
            fprintf(stderr,
                "FAIL: codec_length_ref_dotted_basic test_vector @SCXML L45: "
                "decode left %zu bytes unconsumed\n",
                sce_forge_cursor_remaining(&_cursor));
            ++failures;
        } else {
            if (decoded.carrier != (uint8_t)0x21u) {
                fprintf(stderr,
                    "FAIL: codec_length_ref_dotted_basic test_vector @SCXML L45: "
                    "field `carrier` mismatch\n");
                ++failures;
            }
            {
                static const uint8_t _expected_field[] = { 0xaa, 0xbb };
                if (decoded.payload_len != sizeof _expected_field
                    || memcmp(decoded.payload,
                              _expected_field,
                              decoded.payload_len) != 0) {
                    fprintf(stderr,
                        "FAIL: codec_length_ref_dotted_basic test_vector @SCXML L45: "
                        "field `payload` (bytes) mismatch\n");
                    ++failures;
                }
            }
        }
    }
    {
        codec_length_ref_dotted_basic_t actual = {0};
        actual.carrier = (uint8_t)0xf5u;
        {
            static const uint8_t _bytes[] = { 0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e };
            memcpy(actual.payload, _bytes, sizeof _bytes);
            actual.payload_len = sizeof _bytes;
        }
        uint8_t _encoded_buf[CODEC_LENGTH_REF_DOTTED_BASIC_MAX_BYTES];
        size_t _encoded_len = 0;
        sce_forge_codec_status_t _enc_st = codec_length_ref_dotted_basic_encode_to_buf(&actual, _encoded_buf, sizeof(_encoded_buf), &_encoded_len);
        (void)_enc_st;
        static const uint8_t _expected[] = { 0xf5, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e };
        if (_encoded_len != sizeof _expected
            || memcmp(_encoded_buf, _expected, _encoded_len) != 0) {
            fprintf(stderr,
                "FAIL: codec_length_ref_dotted_basic test_vector @SCXML L49: "
                "encode length=%zu (expected %zu)\n",
                _encoded_len, sizeof _expected);
            ++failures;
        }
        sce_forge_cursor_t _cursor = sce_forge_cursor_init(_expected, sizeof _expected);
        codec_length_ref_dotted_basic_t decoded = {0};
        sce_forge_codec_status_t _st = codec_length_ref_dotted_basic_decode(&_cursor, &decoded);
        if (_st != SCE_FORGE_CODEC_OK) {
            fprintf(stderr,
                "FAIL: codec_length_ref_dotted_basic test_vector @SCXML L49: "
                "decode status=%d\n", (int)_st);
            ++failures;
        } else if (sce_forge_cursor_remaining(&_cursor) != 0) {
            fprintf(stderr,
                "FAIL: codec_length_ref_dotted_basic test_vector @SCXML L49: "
                "decode left %zu bytes unconsumed\n",
                sce_forge_cursor_remaining(&_cursor));
            ++failures;
        } else {
            if (decoded.carrier != (uint8_t)0xf5u) {
                fprintf(stderr,
                    "FAIL: codec_length_ref_dotted_basic test_vector @SCXML L49: "
                    "field `carrier` mismatch\n");
                ++failures;
            }
            {
                static const uint8_t _expected_field[] = { 0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e };
                if (decoded.payload_len != sizeof _expected_field
                    || memcmp(decoded.payload,
                              _expected_field,
                              decoded.payload_len) != 0) {
                    fprintf(stderr,
                        "FAIL: codec_length_ref_dotted_basic test_vector @SCXML L49: "
                        "field `payload` (bytes) mismatch\n");
                    ++failures;
                }
            }
        }
    }
    return failures;
}

#endif  /* SCE_FORGE_CODEC_LENGTH_REF_DOTTED_BASIC_TEST_H */
