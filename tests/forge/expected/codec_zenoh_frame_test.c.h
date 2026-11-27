/* SCE Forge: Auto-generated codec test-vector sidecar (RFC §synth-5-B) */
/* Companion to codec_zenoh_frame.h — do not edit; regenerate from the source SCXML. */

#ifndef SCE_FORGE_CODEC_ZENOH_FRAME_TEST_H
#define SCE_FORGE_CODEC_ZENOH_FRAME_TEST_H

#include <stdint.h>
#include "sce/text.h"
#include "sce/forge/test_vector.h"
#include "codec_zenoh_frame.h"

static inline int test_vector_codec_zenoh_frame(void) {
    int failures = 0;
    /* The one line every failed row's report is built in, in turn. */
    char _line[SCE_FORGE_TEST_VECTOR_LINE_CAP];
    {
        codec_zenoh_frame_t actual = {0};
        actual.sn = (uint64_t)0x0uLL;
        {
            actual.payload_len = 0;
        }
        uint8_t _encoded_buf[CODEC_ZENOH_FRAME_MAX_BYTES];
        size_t _encoded_len = 0;
        sce_forge_codec_status_t _enc_st = codec_zenoh_frame_encode_to_buf(&actual, _encoded_buf, sizeof(_encoded_buf), &_encoded_len);
        (void)_enc_st;
        static const uint8_t _expected[] = { 0x00 };
        if (_encoded_len != sizeof _expected
            || memcmp(_encoded_buf, _expected, _encoded_len) != 0) {
            sce_text_writer_t _report = sce_forge_test_vector_line(_line, sizeof _line, 38u);
            SCE_TEXT_PUT_LITERAL(&_report, "encode length=");
            sce_text_put_u64(&_report, (uint64_t)_encoded_len);
            SCE_TEXT_PUT_LITERAL(&_report, " (expected ");
            sce_text_put_u64(&_report, (uint64_t)sizeof _expected);
            sce_text_put_char(&_report, ')');
            sce_forge_test_vector_failed("codec_zenoh_frame", &_report);
            ++failures;
        }
        sce_forge_cursor_t _cursor = sce_forge_cursor_init(_expected, sizeof _expected);
        codec_zenoh_frame_t decoded = {0};
        sce_forge_codec_status_t _st = codec_zenoh_frame_decode(&_cursor, &decoded);
        if (_st != SCE_FORGE_CODEC_OK) {
            sce_text_writer_t _report = sce_forge_test_vector_line(_line, sizeof _line, 38u);
            SCE_TEXT_PUT_LITERAL(&_report, "decode status=");
            sce_text_put_i64(&_report, (int64_t)_st);
            sce_forge_test_vector_failed("codec_zenoh_frame", &_report);
            ++failures;
        } else if (sce_forge_cursor_remaining(&_cursor) != 0) {
            sce_text_writer_t _report = sce_forge_test_vector_line(_line, sizeof _line, 38u);
            SCE_TEXT_PUT_LITERAL(&_report, "decode left ");
            sce_text_put_u64(&_report, (uint64_t)sce_forge_cursor_remaining(&_cursor));
            SCE_TEXT_PUT_LITERAL(&_report, " bytes unconsumed");
            sce_forge_test_vector_failed("codec_zenoh_frame", &_report);
            ++failures;
        } else {
            if (decoded.sn != (uint64_t)0x0uLL) {
                sce_text_writer_t _report = sce_forge_test_vector_line(_line, sizeof _line, 38u);
                SCE_TEXT_PUT_LITERAL(&_report, "field `sn` mismatch");
                sce_forge_test_vector_failed("codec_zenoh_frame", &_report);
                ++failures;
            }
            {
                if (decoded.payload_len != 0) {
                    sce_text_writer_t _report = sce_forge_test_vector_line(_line, sizeof _line, 38u);
                    SCE_TEXT_PUT_LITERAL(&_report, "field `payload` expected empty bytes, got len=");
                    /* The length companion is `size_t` for a tail field but
                     * the (possibly narrower) wire length-field for a
                     * length-ref payload: widened to the writer's one width. */
                    sce_text_put_u64(&_report, (uint64_t)decoded.payload_len);
                    sce_forge_test_vector_failed("codec_zenoh_frame", &_report);
                    ++failures;
                }
            }
        }
    }
    {
        codec_zenoh_frame_t actual = {0};
        actual.sn = (uint64_t)0x1uLL;
        {
            static const uint8_t _bytes[] = { 0xca, 0xfe };
            memcpy(actual.payload, _bytes, sizeof _bytes);
            actual.payload_len = sizeof _bytes;
        }
        uint8_t _encoded_buf[CODEC_ZENOH_FRAME_MAX_BYTES];
        size_t _encoded_len = 0;
        sce_forge_codec_status_t _enc_st = codec_zenoh_frame_encode_to_buf(&actual, _encoded_buf, sizeof(_encoded_buf), &_encoded_len);
        (void)_enc_st;
        static const uint8_t _expected[] = { 0x01, 0xca, 0xfe };
        if (_encoded_len != sizeof _expected
            || memcmp(_encoded_buf, _expected, _encoded_len) != 0) {
            sce_text_writer_t _report = sce_forge_test_vector_line(_line, sizeof _line, 42u);
            SCE_TEXT_PUT_LITERAL(&_report, "encode length=");
            sce_text_put_u64(&_report, (uint64_t)_encoded_len);
            SCE_TEXT_PUT_LITERAL(&_report, " (expected ");
            sce_text_put_u64(&_report, (uint64_t)sizeof _expected);
            sce_text_put_char(&_report, ')');
            sce_forge_test_vector_failed("codec_zenoh_frame", &_report);
            ++failures;
        }
        sce_forge_cursor_t _cursor = sce_forge_cursor_init(_expected, sizeof _expected);
        codec_zenoh_frame_t decoded = {0};
        sce_forge_codec_status_t _st = codec_zenoh_frame_decode(&_cursor, &decoded);
        if (_st != SCE_FORGE_CODEC_OK) {
            sce_text_writer_t _report = sce_forge_test_vector_line(_line, sizeof _line, 42u);
            SCE_TEXT_PUT_LITERAL(&_report, "decode status=");
            sce_text_put_i64(&_report, (int64_t)_st);
            sce_forge_test_vector_failed("codec_zenoh_frame", &_report);
            ++failures;
        } else if (sce_forge_cursor_remaining(&_cursor) != 0) {
            sce_text_writer_t _report = sce_forge_test_vector_line(_line, sizeof _line, 42u);
            SCE_TEXT_PUT_LITERAL(&_report, "decode left ");
            sce_text_put_u64(&_report, (uint64_t)sce_forge_cursor_remaining(&_cursor));
            SCE_TEXT_PUT_LITERAL(&_report, " bytes unconsumed");
            sce_forge_test_vector_failed("codec_zenoh_frame", &_report);
            ++failures;
        } else {
            if (decoded.sn != (uint64_t)0x1uLL) {
                sce_text_writer_t _report = sce_forge_test_vector_line(_line, sizeof _line, 42u);
                SCE_TEXT_PUT_LITERAL(&_report, "field `sn` mismatch");
                sce_forge_test_vector_failed("codec_zenoh_frame", &_report);
                ++failures;
            }
            {
                static const uint8_t _expected_field[] = { 0xca, 0xfe };
                if (decoded.payload_len != sizeof _expected_field
                    || memcmp(decoded.payload,
                              _expected_field,
                              decoded.payload_len) != 0) {
                    sce_text_writer_t _report = sce_forge_test_vector_line(_line, sizeof _line, 42u);
                    SCE_TEXT_PUT_LITERAL(&_report, "field `payload` (bytes) mismatch");
                    sce_forge_test_vector_failed("codec_zenoh_frame", &_report);
                    ++failures;
                }
            }
        }
    }
    {
        codec_zenoh_frame_t actual = {0};
        actual.sn = (uint64_t)0x7fuLL;
        {
            static const uint8_t _bytes[] = { 0xaa, 0xbb, 0xcc };
            memcpy(actual.payload, _bytes, sizeof _bytes);
            actual.payload_len = sizeof _bytes;
        }
        uint8_t _encoded_buf[CODEC_ZENOH_FRAME_MAX_BYTES];
        size_t _encoded_len = 0;
        sce_forge_codec_status_t _enc_st = codec_zenoh_frame_encode_to_buf(&actual, _encoded_buf, sizeof(_encoded_buf), &_encoded_len);
        (void)_enc_st;
        static const uint8_t _expected[] = { 0x7f, 0xaa, 0xbb, 0xcc };
        if (_encoded_len != sizeof _expected
            || memcmp(_encoded_buf, _expected, _encoded_len) != 0) {
            sce_text_writer_t _report = sce_forge_test_vector_line(_line, sizeof _line, 46u);
            SCE_TEXT_PUT_LITERAL(&_report, "encode length=");
            sce_text_put_u64(&_report, (uint64_t)_encoded_len);
            SCE_TEXT_PUT_LITERAL(&_report, " (expected ");
            sce_text_put_u64(&_report, (uint64_t)sizeof _expected);
            sce_text_put_char(&_report, ')');
            sce_forge_test_vector_failed("codec_zenoh_frame", &_report);
            ++failures;
        }
        sce_forge_cursor_t _cursor = sce_forge_cursor_init(_expected, sizeof _expected);
        codec_zenoh_frame_t decoded = {0};
        sce_forge_codec_status_t _st = codec_zenoh_frame_decode(&_cursor, &decoded);
        if (_st != SCE_FORGE_CODEC_OK) {
            sce_text_writer_t _report = sce_forge_test_vector_line(_line, sizeof _line, 46u);
            SCE_TEXT_PUT_LITERAL(&_report, "decode status=");
            sce_text_put_i64(&_report, (int64_t)_st);
            sce_forge_test_vector_failed("codec_zenoh_frame", &_report);
            ++failures;
        } else if (sce_forge_cursor_remaining(&_cursor) != 0) {
            sce_text_writer_t _report = sce_forge_test_vector_line(_line, sizeof _line, 46u);
            SCE_TEXT_PUT_LITERAL(&_report, "decode left ");
            sce_text_put_u64(&_report, (uint64_t)sce_forge_cursor_remaining(&_cursor));
            SCE_TEXT_PUT_LITERAL(&_report, " bytes unconsumed");
            sce_forge_test_vector_failed("codec_zenoh_frame", &_report);
            ++failures;
        } else {
            if (decoded.sn != (uint64_t)0x7fuLL) {
                sce_text_writer_t _report = sce_forge_test_vector_line(_line, sizeof _line, 46u);
                SCE_TEXT_PUT_LITERAL(&_report, "field `sn` mismatch");
                sce_forge_test_vector_failed("codec_zenoh_frame", &_report);
                ++failures;
            }
            {
                static const uint8_t _expected_field[] = { 0xaa, 0xbb, 0xcc };
                if (decoded.payload_len != sizeof _expected_field
                    || memcmp(decoded.payload,
                              _expected_field,
                              decoded.payload_len) != 0) {
                    sce_text_writer_t _report = sce_forge_test_vector_line(_line, sizeof _line, 46u);
                    SCE_TEXT_PUT_LITERAL(&_report, "field `payload` (bytes) mismatch");
                    sce_forge_test_vector_failed("codec_zenoh_frame", &_report);
                    ++failures;
                }
            }
        }
    }
    {
        codec_zenoh_frame_t actual = {0};
        actual.sn = (uint64_t)0x80uLL;
        {
            static const uint8_t _bytes[] = { 0xde, 0xad };
            memcpy(actual.payload, _bytes, sizeof _bytes);
            actual.payload_len = sizeof _bytes;
        }
        uint8_t _encoded_buf[CODEC_ZENOH_FRAME_MAX_BYTES];
        size_t _encoded_len = 0;
        sce_forge_codec_status_t _enc_st = codec_zenoh_frame_encode_to_buf(&actual, _encoded_buf, sizeof(_encoded_buf), &_encoded_len);
        (void)_enc_st;
        static const uint8_t _expected[] = { 0x80, 0x01, 0xde, 0xad };
        if (_encoded_len != sizeof _expected
            || memcmp(_encoded_buf, _expected, _encoded_len) != 0) {
            sce_text_writer_t _report = sce_forge_test_vector_line(_line, sizeof _line, 50u);
            SCE_TEXT_PUT_LITERAL(&_report, "encode length=");
            sce_text_put_u64(&_report, (uint64_t)_encoded_len);
            SCE_TEXT_PUT_LITERAL(&_report, " (expected ");
            sce_text_put_u64(&_report, (uint64_t)sizeof _expected);
            sce_text_put_char(&_report, ')');
            sce_forge_test_vector_failed("codec_zenoh_frame", &_report);
            ++failures;
        }
        sce_forge_cursor_t _cursor = sce_forge_cursor_init(_expected, sizeof _expected);
        codec_zenoh_frame_t decoded = {0};
        sce_forge_codec_status_t _st = codec_zenoh_frame_decode(&_cursor, &decoded);
        if (_st != SCE_FORGE_CODEC_OK) {
            sce_text_writer_t _report = sce_forge_test_vector_line(_line, sizeof _line, 50u);
            SCE_TEXT_PUT_LITERAL(&_report, "decode status=");
            sce_text_put_i64(&_report, (int64_t)_st);
            sce_forge_test_vector_failed("codec_zenoh_frame", &_report);
            ++failures;
        } else if (sce_forge_cursor_remaining(&_cursor) != 0) {
            sce_text_writer_t _report = sce_forge_test_vector_line(_line, sizeof _line, 50u);
            SCE_TEXT_PUT_LITERAL(&_report, "decode left ");
            sce_text_put_u64(&_report, (uint64_t)sce_forge_cursor_remaining(&_cursor));
            SCE_TEXT_PUT_LITERAL(&_report, " bytes unconsumed");
            sce_forge_test_vector_failed("codec_zenoh_frame", &_report);
            ++failures;
        } else {
            if (decoded.sn != (uint64_t)0x80uLL) {
                sce_text_writer_t _report = sce_forge_test_vector_line(_line, sizeof _line, 50u);
                SCE_TEXT_PUT_LITERAL(&_report, "field `sn` mismatch");
                sce_forge_test_vector_failed("codec_zenoh_frame", &_report);
                ++failures;
            }
            {
                static const uint8_t _expected_field[] = { 0xde, 0xad };
                if (decoded.payload_len != sizeof _expected_field
                    || memcmp(decoded.payload,
                              _expected_field,
                              decoded.payload_len) != 0) {
                    sce_text_writer_t _report = sce_forge_test_vector_line(_line, sizeof _line, 50u);
                    SCE_TEXT_PUT_LITERAL(&_report, "field `payload` (bytes) mismatch");
                    sce_forge_test_vector_failed("codec_zenoh_frame", &_report);
                    ++failures;
                }
            }
        }
    }
    return failures;
}

#endif  /* SCE_FORGE_CODEC_ZENOH_FRAME_TEST_H */
