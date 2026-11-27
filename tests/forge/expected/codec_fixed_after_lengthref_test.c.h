/* SCE Forge: Auto-generated codec test-vector sidecar (RFC §synth-5-B) */
/* Companion to codec_fixed_after_lengthref.h — do not edit; regenerate from the source SCXML. */

#ifndef SCE_FORGE_CODEC_FIXED_AFTER_LENGTHREF_TEST_H
#define SCE_FORGE_CODEC_FIXED_AFTER_LENGTHREF_TEST_H

#include <stdint.h>
#include "sce/text.h"
#include "sce/forge/test_vector.h"
#include "codec_fixed_after_lengthref.h"

static inline int test_vector_codec_fixed_after_lengthref(void) {
    int failures = 0;
    /* The one line every failed row's report is built in, in turn. */
    char _line[SCE_FORGE_TEST_VECTOR_LINE_CAP];
    {
        codec_fixed_after_lengthref_t actual = {0};
        actual.header = (uint8_t)0xaau;
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
            sce_text_writer_t _report = sce_forge_test_vector_line(_line, sizeof _line, 30u);
            SCE_TEXT_PUT_LITERAL(&_report, "encode length=");
            sce_text_put_u64(&_report, (uint64_t)_encoded_len);
            SCE_TEXT_PUT_LITERAL(&_report, " (expected ");
            sce_text_put_u64(&_report, (uint64_t)sizeof _expected);
            sce_text_put_char(&_report, ')');
            sce_forge_test_vector_failed("codec_fixed_after_lengthref", &_report);
            ++failures;
        }
        sce_forge_cursor_t _cursor = sce_forge_cursor_init(_expected, sizeof _expected);
        codec_fixed_after_lengthref_t decoded = {0};
        sce_forge_codec_status_t _st = codec_fixed_after_lengthref_decode(&_cursor, &decoded);
        if (_st != SCE_FORGE_CODEC_OK) {
            sce_text_writer_t _report = sce_forge_test_vector_line(_line, sizeof _line, 30u);
            SCE_TEXT_PUT_LITERAL(&_report, "decode status=");
            sce_text_put_i64(&_report, (int64_t)_st);
            sce_forge_test_vector_failed("codec_fixed_after_lengthref", &_report);
            ++failures;
        } else if (sce_forge_cursor_remaining(&_cursor) != 0) {
            sce_text_writer_t _report = sce_forge_test_vector_line(_line, sizeof _line, 30u);
            SCE_TEXT_PUT_LITERAL(&_report, "decode left ");
            sce_text_put_u64(&_report, (uint64_t)sce_forge_cursor_remaining(&_cursor));
            SCE_TEXT_PUT_LITERAL(&_report, " bytes unconsumed");
            sce_forge_test_vector_failed("codec_fixed_after_lengthref", &_report);
            ++failures;
        } else {
            if (decoded.header != (uint8_t)0xaau) {
                sce_text_writer_t _report = sce_forge_test_vector_line(_line, sizeof _line, 30u);
                SCE_TEXT_PUT_LITERAL(&_report, "field `header` mismatch");
                sce_forge_test_vector_failed("codec_fixed_after_lengthref", &_report);
                ++failures;
            }
            {
                static const uint8_t _expected_field[] = { 0xde, 0xad, 0xbe };
                if (decoded.payload_len != sizeof _expected_field
                    || memcmp(decoded.payload,
                              _expected_field,
                              decoded.payload_len) != 0) {
                    sce_text_writer_t _report = sce_forge_test_vector_line(_line, sizeof _line, 30u);
                    SCE_TEXT_PUT_LITERAL(&_report, "field `payload` (bytes) mismatch");
                    sce_forge_test_vector_failed("codec_fixed_after_lengthref", &_report);
                    ++failures;
                }
            }
            if (decoded.crc32 != (uint32_t)0x11223344u) {
                sce_text_writer_t _report = sce_forge_test_vector_line(_line, sizeof _line, 30u);
                SCE_TEXT_PUT_LITERAL(&_report, "field `crc32` mismatch");
                sce_forge_test_vector_failed("codec_fixed_after_lengthref", &_report);
                ++failures;
            }
        }
    }
    {
        codec_fixed_after_lengthref_t actual = {0};
        actual.header = (uint8_t)0x1u;
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
            sce_text_writer_t _report = sce_forge_test_vector_line(_line, sizeof _line, 37u);
            SCE_TEXT_PUT_LITERAL(&_report, "encode length=");
            sce_text_put_u64(&_report, (uint64_t)_encoded_len);
            SCE_TEXT_PUT_LITERAL(&_report, " (expected ");
            sce_text_put_u64(&_report, (uint64_t)sizeof _expected);
            sce_text_put_char(&_report, ')');
            sce_forge_test_vector_failed("codec_fixed_after_lengthref", &_report);
            ++failures;
        }
        sce_forge_cursor_t _cursor = sce_forge_cursor_init(_expected, sizeof _expected);
        codec_fixed_after_lengthref_t decoded = {0};
        sce_forge_codec_status_t _st = codec_fixed_after_lengthref_decode(&_cursor, &decoded);
        if (_st != SCE_FORGE_CODEC_OK) {
            sce_text_writer_t _report = sce_forge_test_vector_line(_line, sizeof _line, 37u);
            SCE_TEXT_PUT_LITERAL(&_report, "decode status=");
            sce_text_put_i64(&_report, (int64_t)_st);
            sce_forge_test_vector_failed("codec_fixed_after_lengthref", &_report);
            ++failures;
        } else if (sce_forge_cursor_remaining(&_cursor) != 0) {
            sce_text_writer_t _report = sce_forge_test_vector_line(_line, sizeof _line, 37u);
            SCE_TEXT_PUT_LITERAL(&_report, "decode left ");
            sce_text_put_u64(&_report, (uint64_t)sce_forge_cursor_remaining(&_cursor));
            SCE_TEXT_PUT_LITERAL(&_report, " bytes unconsumed");
            sce_forge_test_vector_failed("codec_fixed_after_lengthref", &_report);
            ++failures;
        } else {
            if (decoded.header != (uint8_t)0x1u) {
                sce_text_writer_t _report = sce_forge_test_vector_line(_line, sizeof _line, 37u);
                SCE_TEXT_PUT_LITERAL(&_report, "field `header` mismatch");
                sce_forge_test_vector_failed("codec_fixed_after_lengthref", &_report);
                ++failures;
            }
            {
                if (decoded.payload_len != 0) {
                    sce_text_writer_t _report = sce_forge_test_vector_line(_line, sizeof _line, 37u);
                    SCE_TEXT_PUT_LITERAL(&_report, "field `payload` expected empty bytes, got len=");
                    /* The length companion is `size_t` for a tail field but
                     * the (possibly narrower) wire length-field for a
                     * length-ref payload: widened to the writer's one width. */
                    sce_text_put_u64(&_report, (uint64_t)decoded.payload_len);
                    sce_forge_test_vector_failed("codec_fixed_after_lengthref", &_report);
                    ++failures;
                }
            }
            if (decoded.crc32 != (uint32_t)0xcafebabeu) {
                sce_text_writer_t _report = sce_forge_test_vector_line(_line, sizeof _line, 37u);
                SCE_TEXT_PUT_LITERAL(&_report, "field `crc32` mismatch");
                sce_forge_test_vector_failed("codec_fixed_after_lengthref", &_report);
                ++failures;
            }
        }
    }
    {
        codec_fixed_after_lengthref_t actual = {0};
        actual.header = (uint8_t)0xffu;
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
            sce_text_writer_t _report = sce_forge_test_vector_line(_line, sizeof _line, 44u);
            SCE_TEXT_PUT_LITERAL(&_report, "encode length=");
            sce_text_put_u64(&_report, (uint64_t)_encoded_len);
            SCE_TEXT_PUT_LITERAL(&_report, " (expected ");
            sce_text_put_u64(&_report, (uint64_t)sizeof _expected);
            sce_text_put_char(&_report, ')');
            sce_forge_test_vector_failed("codec_fixed_after_lengthref", &_report);
            ++failures;
        }
        sce_forge_cursor_t _cursor = sce_forge_cursor_init(_expected, sizeof _expected);
        codec_fixed_after_lengthref_t decoded = {0};
        sce_forge_codec_status_t _st = codec_fixed_after_lengthref_decode(&_cursor, &decoded);
        if (_st != SCE_FORGE_CODEC_OK) {
            sce_text_writer_t _report = sce_forge_test_vector_line(_line, sizeof _line, 44u);
            SCE_TEXT_PUT_LITERAL(&_report, "decode status=");
            sce_text_put_i64(&_report, (int64_t)_st);
            sce_forge_test_vector_failed("codec_fixed_after_lengthref", &_report);
            ++failures;
        } else if (sce_forge_cursor_remaining(&_cursor) != 0) {
            sce_text_writer_t _report = sce_forge_test_vector_line(_line, sizeof _line, 44u);
            SCE_TEXT_PUT_LITERAL(&_report, "decode left ");
            sce_text_put_u64(&_report, (uint64_t)sce_forge_cursor_remaining(&_cursor));
            SCE_TEXT_PUT_LITERAL(&_report, " bytes unconsumed");
            sce_forge_test_vector_failed("codec_fixed_after_lengthref", &_report);
            ++failures;
        } else {
            if (decoded.header != (uint8_t)0xffu) {
                sce_text_writer_t _report = sce_forge_test_vector_line(_line, sizeof _line, 44u);
                SCE_TEXT_PUT_LITERAL(&_report, "field `header` mismatch");
                sce_forge_test_vector_failed("codec_fixed_after_lengthref", &_report);
                ++failures;
            }
            {
                static const uint8_t _expected_field[] = { 0x01, 0x02, 0x03, 0x04, 0x05 };
                if (decoded.payload_len != sizeof _expected_field
                    || memcmp(decoded.payload,
                              _expected_field,
                              decoded.payload_len) != 0) {
                    sce_text_writer_t _report = sce_forge_test_vector_line(_line, sizeof _line, 44u);
                    SCE_TEXT_PUT_LITERAL(&_report, "field `payload` (bytes) mismatch");
                    sce_forge_test_vector_failed("codec_fixed_after_lengthref", &_report);
                    ++failures;
                }
            }
            if (decoded.crc32 != (uint32_t)0x1u) {
                sce_text_writer_t _report = sce_forge_test_vector_line(_line, sizeof _line, 44u);
                SCE_TEXT_PUT_LITERAL(&_report, "field `crc32` mismatch");
                sce_forge_test_vector_failed("codec_fixed_after_lengthref", &_report);
                ++failures;
            }
        }
    }
    return failures;
}

#endif  /* SCE_FORGE_CODEC_FIXED_AFTER_LENGTHREF_TEST_H */
