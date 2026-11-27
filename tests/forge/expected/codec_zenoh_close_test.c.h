/* SCE Forge: Auto-generated codec test-vector sidecar (RFC §synth-5-B) */
/* Companion to codec_zenoh_close.h — do not edit; regenerate from the source SCXML. */

#ifndef SCE_FORGE_CODEC_ZENOH_CLOSE_TEST_H
#define SCE_FORGE_CODEC_ZENOH_CLOSE_TEST_H

#include <stdint.h>
#include "sce/text.h"
#include "sce/forge/test_vector.h"
#include "codec_zenoh_close.h"

static inline int test_vector_codec_zenoh_close(void) {
    int failures = 0;
    /* The one line every failed row's report is built in, in turn. */
    char _line[SCE_FORGE_TEST_VECTOR_LINE_CAP];
    {
        codec_zenoh_close_t actual = {0};
        actual.reason = (uint8_t)0x0u;
        uint8_t _encoded_buf[CODEC_ZENOH_CLOSE_MAX_BYTES];
        size_t _encoded_len = 0;
        sce_forge_codec_status_t _enc_st = codec_zenoh_close_encode_to_buf(&actual, _encoded_buf, sizeof(_encoded_buf), &_encoded_len);
        (void)_enc_st;
        static const uint8_t _expected[] = { 0x00 };
        if (_encoded_len != sizeof _expected
            || memcmp(_encoded_buf, _expected, _encoded_len) != 0) {
            sce_text_writer_t _report = sce_forge_test_vector_line(_line, sizeof _line, 27u);
            SCE_TEXT_PUT_LITERAL(&_report, "encode length=");
            sce_text_put_u64(&_report, (uint64_t)_encoded_len);
            SCE_TEXT_PUT_LITERAL(&_report, " (expected ");
            sce_text_put_u64(&_report, (uint64_t)sizeof _expected);
            sce_text_put_char(&_report, ')');
            sce_forge_test_vector_failed("codec_zenoh_close", &_report);
            ++failures;
        }
        sce_forge_cursor_t _cursor = sce_forge_cursor_init(_expected, sizeof _expected);
        codec_zenoh_close_t decoded = {0};
        sce_forge_codec_status_t _st = codec_zenoh_close_decode(&_cursor, &decoded);
        if (_st != SCE_FORGE_CODEC_OK) {
            sce_text_writer_t _report = sce_forge_test_vector_line(_line, sizeof _line, 27u);
            SCE_TEXT_PUT_LITERAL(&_report, "decode status=");
            sce_text_put_i64(&_report, (int64_t)_st);
            sce_forge_test_vector_failed("codec_zenoh_close", &_report);
            ++failures;
        } else if (sce_forge_cursor_remaining(&_cursor) != 0) {
            sce_text_writer_t _report = sce_forge_test_vector_line(_line, sizeof _line, 27u);
            SCE_TEXT_PUT_LITERAL(&_report, "decode left ");
            sce_text_put_u64(&_report, (uint64_t)sce_forge_cursor_remaining(&_cursor));
            SCE_TEXT_PUT_LITERAL(&_report, " bytes unconsumed");
            sce_forge_test_vector_failed("codec_zenoh_close", &_report);
            ++failures;
        } else {
            if (decoded.reason != (uint8_t)0x0u) {
                sce_text_writer_t _report = sce_forge_test_vector_line(_line, sizeof _line, 27u);
                SCE_TEXT_PUT_LITERAL(&_report, "field `reason` mismatch");
                sce_forge_test_vector_failed("codec_zenoh_close", &_report);
                ++failures;
            }
        }
    }
    {
        codec_zenoh_close_t actual = {0};
        actual.reason = (uint8_t)0x1u;
        uint8_t _encoded_buf[CODEC_ZENOH_CLOSE_MAX_BYTES];
        size_t _encoded_len = 0;
        sce_forge_codec_status_t _enc_st = codec_zenoh_close_encode_to_buf(&actual, _encoded_buf, sizeof(_encoded_buf), &_encoded_len);
        (void)_enc_st;
        static const uint8_t _expected[] = { 0x01 };
        if (_encoded_len != sizeof _expected
            || memcmp(_encoded_buf, _expected, _encoded_len) != 0) {
            sce_text_writer_t _report = sce_forge_test_vector_line(_line, sizeof _line, 30u);
            SCE_TEXT_PUT_LITERAL(&_report, "encode length=");
            sce_text_put_u64(&_report, (uint64_t)_encoded_len);
            SCE_TEXT_PUT_LITERAL(&_report, " (expected ");
            sce_text_put_u64(&_report, (uint64_t)sizeof _expected);
            sce_text_put_char(&_report, ')');
            sce_forge_test_vector_failed("codec_zenoh_close", &_report);
            ++failures;
        }
        sce_forge_cursor_t _cursor = sce_forge_cursor_init(_expected, sizeof _expected);
        codec_zenoh_close_t decoded = {0};
        sce_forge_codec_status_t _st = codec_zenoh_close_decode(&_cursor, &decoded);
        if (_st != SCE_FORGE_CODEC_OK) {
            sce_text_writer_t _report = sce_forge_test_vector_line(_line, sizeof _line, 30u);
            SCE_TEXT_PUT_LITERAL(&_report, "decode status=");
            sce_text_put_i64(&_report, (int64_t)_st);
            sce_forge_test_vector_failed("codec_zenoh_close", &_report);
            ++failures;
        } else if (sce_forge_cursor_remaining(&_cursor) != 0) {
            sce_text_writer_t _report = sce_forge_test_vector_line(_line, sizeof _line, 30u);
            SCE_TEXT_PUT_LITERAL(&_report, "decode left ");
            sce_text_put_u64(&_report, (uint64_t)sce_forge_cursor_remaining(&_cursor));
            SCE_TEXT_PUT_LITERAL(&_report, " bytes unconsumed");
            sce_forge_test_vector_failed("codec_zenoh_close", &_report);
            ++failures;
        } else {
            if (decoded.reason != (uint8_t)0x1u) {
                sce_text_writer_t _report = sce_forge_test_vector_line(_line, sizeof _line, 30u);
                SCE_TEXT_PUT_LITERAL(&_report, "field `reason` mismatch");
                sce_forge_test_vector_failed("codec_zenoh_close", &_report);
                ++failures;
            }
        }
    }
    {
        codec_zenoh_close_t actual = {0};
        actual.reason = (uint8_t)0x2u;
        uint8_t _encoded_buf[CODEC_ZENOH_CLOSE_MAX_BYTES];
        size_t _encoded_len = 0;
        sce_forge_codec_status_t _enc_st = codec_zenoh_close_encode_to_buf(&actual, _encoded_buf, sizeof(_encoded_buf), &_encoded_len);
        (void)_enc_st;
        static const uint8_t _expected[] = { 0x02 };
        if (_encoded_len != sizeof _expected
            || memcmp(_encoded_buf, _expected, _encoded_len) != 0) {
            sce_text_writer_t _report = sce_forge_test_vector_line(_line, sizeof _line, 33u);
            SCE_TEXT_PUT_LITERAL(&_report, "encode length=");
            sce_text_put_u64(&_report, (uint64_t)_encoded_len);
            SCE_TEXT_PUT_LITERAL(&_report, " (expected ");
            sce_text_put_u64(&_report, (uint64_t)sizeof _expected);
            sce_text_put_char(&_report, ')');
            sce_forge_test_vector_failed("codec_zenoh_close", &_report);
            ++failures;
        }
        sce_forge_cursor_t _cursor = sce_forge_cursor_init(_expected, sizeof _expected);
        codec_zenoh_close_t decoded = {0};
        sce_forge_codec_status_t _st = codec_zenoh_close_decode(&_cursor, &decoded);
        if (_st != SCE_FORGE_CODEC_OK) {
            sce_text_writer_t _report = sce_forge_test_vector_line(_line, sizeof _line, 33u);
            SCE_TEXT_PUT_LITERAL(&_report, "decode status=");
            sce_text_put_i64(&_report, (int64_t)_st);
            sce_forge_test_vector_failed("codec_zenoh_close", &_report);
            ++failures;
        } else if (sce_forge_cursor_remaining(&_cursor) != 0) {
            sce_text_writer_t _report = sce_forge_test_vector_line(_line, sizeof _line, 33u);
            SCE_TEXT_PUT_LITERAL(&_report, "decode left ");
            sce_text_put_u64(&_report, (uint64_t)sce_forge_cursor_remaining(&_cursor));
            SCE_TEXT_PUT_LITERAL(&_report, " bytes unconsumed");
            sce_forge_test_vector_failed("codec_zenoh_close", &_report);
            ++failures;
        } else {
            if (decoded.reason != (uint8_t)0x2u) {
                sce_text_writer_t _report = sce_forge_test_vector_line(_line, sizeof _line, 33u);
                SCE_TEXT_PUT_LITERAL(&_report, "field `reason` mismatch");
                sce_forge_test_vector_failed("codec_zenoh_close", &_report);
                ++failures;
            }
        }
    }
    {
        codec_zenoh_close_t actual = {0};
        actual.reason = (uint8_t)0xffu;
        uint8_t _encoded_buf[CODEC_ZENOH_CLOSE_MAX_BYTES];
        size_t _encoded_len = 0;
        sce_forge_codec_status_t _enc_st = codec_zenoh_close_encode_to_buf(&actual, _encoded_buf, sizeof(_encoded_buf), &_encoded_len);
        (void)_enc_st;
        static const uint8_t _expected[] = { 0xff };
        if (_encoded_len != sizeof _expected
            || memcmp(_encoded_buf, _expected, _encoded_len) != 0) {
            sce_text_writer_t _report = sce_forge_test_vector_line(_line, sizeof _line, 36u);
            SCE_TEXT_PUT_LITERAL(&_report, "encode length=");
            sce_text_put_u64(&_report, (uint64_t)_encoded_len);
            SCE_TEXT_PUT_LITERAL(&_report, " (expected ");
            sce_text_put_u64(&_report, (uint64_t)sizeof _expected);
            sce_text_put_char(&_report, ')');
            sce_forge_test_vector_failed("codec_zenoh_close", &_report);
            ++failures;
        }
        sce_forge_cursor_t _cursor = sce_forge_cursor_init(_expected, sizeof _expected);
        codec_zenoh_close_t decoded = {0};
        sce_forge_codec_status_t _st = codec_zenoh_close_decode(&_cursor, &decoded);
        if (_st != SCE_FORGE_CODEC_OK) {
            sce_text_writer_t _report = sce_forge_test_vector_line(_line, sizeof _line, 36u);
            SCE_TEXT_PUT_LITERAL(&_report, "decode status=");
            sce_text_put_i64(&_report, (int64_t)_st);
            sce_forge_test_vector_failed("codec_zenoh_close", &_report);
            ++failures;
        } else if (sce_forge_cursor_remaining(&_cursor) != 0) {
            sce_text_writer_t _report = sce_forge_test_vector_line(_line, sizeof _line, 36u);
            SCE_TEXT_PUT_LITERAL(&_report, "decode left ");
            sce_text_put_u64(&_report, (uint64_t)sce_forge_cursor_remaining(&_cursor));
            SCE_TEXT_PUT_LITERAL(&_report, " bytes unconsumed");
            sce_forge_test_vector_failed("codec_zenoh_close", &_report);
            ++failures;
        } else {
            if (decoded.reason != (uint8_t)0xffu) {
                sce_text_writer_t _report = sce_forge_test_vector_line(_line, sizeof _line, 36u);
                SCE_TEXT_PUT_LITERAL(&_report, "field `reason` mismatch");
                sce_forge_test_vector_failed("codec_zenoh_close", &_report);
                ++failures;
            }
        }
    }
    return failures;
}

#endif  /* SCE_FORGE_CODEC_ZENOH_CLOSE_TEST_H */
