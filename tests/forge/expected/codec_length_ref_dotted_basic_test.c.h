/* SCE Forge: Auto-generated codec test-vector sidecar (RFC §synth-5-B) */
/* Companion to codec_length_ref_dotted_basic.h — do not edit; regenerate from the source SCXML. */

#ifndef SCE_FORGE_CODEC_LENGTH_REF_DOTTED_BASIC_TEST_H
#define SCE_FORGE_CODEC_LENGTH_REF_DOTTED_BASIC_TEST_H

#include <stdint.h>
#include "sce/text.h"
#include "sce/forge/test_vector.h"
#include "codec_length_ref_dotted_basic.h"

static inline int test_vector_codec_length_ref_dotted_basic(void) {
    int failures = 0;
    /* The one line every failed row's report is built in, in turn. */
    char _line[SCE_FORGE_TEST_VECTOR_LINE_CAP];
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
            sce_text_writer_t _report = sce_forge_test_vector_line(_line, sizeof _line, 41u);
            SCE_TEXT_PUT_LITERAL(&_report, "encode length=");
            sce_text_put_u64(&_report, (uint64_t)_encoded_len);
            SCE_TEXT_PUT_LITERAL(&_report, " (expected ");
            sce_text_put_u64(&_report, (uint64_t)sizeof _expected);
            sce_text_put_char(&_report, ')');
            sce_forge_test_vector_failed("codec_length_ref_dotted_basic", &_report);
            ++failures;
        }
        sce_forge_cursor_t _cursor = sce_forge_cursor_init(_expected, sizeof _expected);
        codec_length_ref_dotted_basic_t decoded = {0};
        sce_forge_codec_status_t _st = codec_length_ref_dotted_basic_decode(&_cursor, &decoded);
        if (_st != SCE_FORGE_CODEC_OK) {
            sce_text_writer_t _report = sce_forge_test_vector_line(_line, sizeof _line, 41u);
            SCE_TEXT_PUT_LITERAL(&_report, "decode status=");
            sce_text_put_i64(&_report, (int64_t)_st);
            sce_forge_test_vector_failed("codec_length_ref_dotted_basic", &_report);
            ++failures;
        } else if (sce_forge_cursor_remaining(&_cursor) != 0) {
            sce_text_writer_t _report = sce_forge_test_vector_line(_line, sizeof _line, 41u);
            SCE_TEXT_PUT_LITERAL(&_report, "decode left ");
            sce_text_put_u64(&_report, (uint64_t)sce_forge_cursor_remaining(&_cursor));
            SCE_TEXT_PUT_LITERAL(&_report, " bytes unconsumed");
            sce_forge_test_vector_failed("codec_length_ref_dotted_basic", &_report);
            ++failures;
        } else {
            if (decoded.carrier != (uint8_t)0x0u) {
                sce_text_writer_t _report = sce_forge_test_vector_line(_line, sizeof _line, 41u);
                SCE_TEXT_PUT_LITERAL(&_report, "field `carrier` mismatch");
                sce_forge_test_vector_failed("codec_length_ref_dotted_basic", &_report);
                ++failures;
            }
            {
                if (decoded.payload_len != 0) {
                    sce_text_writer_t _report = sce_forge_test_vector_line(_line, sizeof _line, 41u);
                    SCE_TEXT_PUT_LITERAL(&_report, "field `payload` expected empty bytes, got len=");
                    /* The length companion is `size_t` for a tail field but
                     * the (possibly narrower) wire length-field for a
                     * length-ref payload: widened to the writer's one width. */
                    sce_text_put_u64(&_report, (uint64_t)decoded.payload_len);
                    sce_forge_test_vector_failed("codec_length_ref_dotted_basic", &_report);
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
            sce_text_writer_t _report = sce_forge_test_vector_line(_line, sizeof _line, 45u);
            SCE_TEXT_PUT_LITERAL(&_report, "encode length=");
            sce_text_put_u64(&_report, (uint64_t)_encoded_len);
            SCE_TEXT_PUT_LITERAL(&_report, " (expected ");
            sce_text_put_u64(&_report, (uint64_t)sizeof _expected);
            sce_text_put_char(&_report, ')');
            sce_forge_test_vector_failed("codec_length_ref_dotted_basic", &_report);
            ++failures;
        }
        sce_forge_cursor_t _cursor = sce_forge_cursor_init(_expected, sizeof _expected);
        codec_length_ref_dotted_basic_t decoded = {0};
        sce_forge_codec_status_t _st = codec_length_ref_dotted_basic_decode(&_cursor, &decoded);
        if (_st != SCE_FORGE_CODEC_OK) {
            sce_text_writer_t _report = sce_forge_test_vector_line(_line, sizeof _line, 45u);
            SCE_TEXT_PUT_LITERAL(&_report, "decode status=");
            sce_text_put_i64(&_report, (int64_t)_st);
            sce_forge_test_vector_failed("codec_length_ref_dotted_basic", &_report);
            ++failures;
        } else if (sce_forge_cursor_remaining(&_cursor) != 0) {
            sce_text_writer_t _report = sce_forge_test_vector_line(_line, sizeof _line, 45u);
            SCE_TEXT_PUT_LITERAL(&_report, "decode left ");
            sce_text_put_u64(&_report, (uint64_t)sce_forge_cursor_remaining(&_cursor));
            SCE_TEXT_PUT_LITERAL(&_report, " bytes unconsumed");
            sce_forge_test_vector_failed("codec_length_ref_dotted_basic", &_report);
            ++failures;
        } else {
            if (decoded.carrier != (uint8_t)0x21u) {
                sce_text_writer_t _report = sce_forge_test_vector_line(_line, sizeof _line, 45u);
                SCE_TEXT_PUT_LITERAL(&_report, "field `carrier` mismatch");
                sce_forge_test_vector_failed("codec_length_ref_dotted_basic", &_report);
                ++failures;
            }
            {
                static const uint8_t _expected_field[] = { 0xaa, 0xbb };
                if (decoded.payload_len != sizeof _expected_field
                    || memcmp(decoded.payload,
                              _expected_field,
                              decoded.payload_len) != 0) {
                    sce_text_writer_t _report = sce_forge_test_vector_line(_line, sizeof _line, 45u);
                    SCE_TEXT_PUT_LITERAL(&_report, "field `payload` (bytes) mismatch");
                    sce_forge_test_vector_failed("codec_length_ref_dotted_basic", &_report);
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
            sce_text_writer_t _report = sce_forge_test_vector_line(_line, sizeof _line, 49u);
            SCE_TEXT_PUT_LITERAL(&_report, "encode length=");
            sce_text_put_u64(&_report, (uint64_t)_encoded_len);
            SCE_TEXT_PUT_LITERAL(&_report, " (expected ");
            sce_text_put_u64(&_report, (uint64_t)sizeof _expected);
            sce_text_put_char(&_report, ')');
            sce_forge_test_vector_failed("codec_length_ref_dotted_basic", &_report);
            ++failures;
        }
        sce_forge_cursor_t _cursor = sce_forge_cursor_init(_expected, sizeof _expected);
        codec_length_ref_dotted_basic_t decoded = {0};
        sce_forge_codec_status_t _st = codec_length_ref_dotted_basic_decode(&_cursor, &decoded);
        if (_st != SCE_FORGE_CODEC_OK) {
            sce_text_writer_t _report = sce_forge_test_vector_line(_line, sizeof _line, 49u);
            SCE_TEXT_PUT_LITERAL(&_report, "decode status=");
            sce_text_put_i64(&_report, (int64_t)_st);
            sce_forge_test_vector_failed("codec_length_ref_dotted_basic", &_report);
            ++failures;
        } else if (sce_forge_cursor_remaining(&_cursor) != 0) {
            sce_text_writer_t _report = sce_forge_test_vector_line(_line, sizeof _line, 49u);
            SCE_TEXT_PUT_LITERAL(&_report, "decode left ");
            sce_text_put_u64(&_report, (uint64_t)sce_forge_cursor_remaining(&_cursor));
            SCE_TEXT_PUT_LITERAL(&_report, " bytes unconsumed");
            sce_forge_test_vector_failed("codec_length_ref_dotted_basic", &_report);
            ++failures;
        } else {
            if (decoded.carrier != (uint8_t)0xf5u) {
                sce_text_writer_t _report = sce_forge_test_vector_line(_line, sizeof _line, 49u);
                SCE_TEXT_PUT_LITERAL(&_report, "field `carrier` mismatch");
                sce_forge_test_vector_failed("codec_length_ref_dotted_basic", &_report);
                ++failures;
            }
            {
                static const uint8_t _expected_field[] = { 0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e };
                if (decoded.payload_len != sizeof _expected_field
                    || memcmp(decoded.payload,
                              _expected_field,
                              decoded.payload_len) != 0) {
                    sce_text_writer_t _report = sce_forge_test_vector_line(_line, sizeof _line, 49u);
                    SCE_TEXT_PUT_LITERAL(&_report, "field `payload` (bytes) mismatch");
                    sce_forge_test_vector_failed("codec_length_ref_dotted_basic", &_report);
                    ++failures;
                }
            }
        }
    }
    return failures;
}

#endif  /* SCE_FORGE_CODEC_LENGTH_REF_DOTTED_BASIC_TEST_H */
