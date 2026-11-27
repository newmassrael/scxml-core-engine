/* SCE Forge: Auto-generated codec test-vector sidecar (RFC §synth-5-B) */
/* Companion to codec_zenoh_locator.h — do not edit; regenerate from the source SCXML. */

#ifndef SCE_FORGE_CODEC_ZENOH_LOCATOR_TEST_H
#define SCE_FORGE_CODEC_ZENOH_LOCATOR_TEST_H

#include <stdint.h>
#include "sce/text.h"
#include "sce/forge/test_vector.h"
#include "codec_zenoh_locator.h"

static inline int test_vector_codec_zenoh_locator(void) {
    int failures = 0;
    /* The one line every failed row's report is built in, in turn. */
    char _line[SCE_FORGE_TEST_VECTOR_LINE_CAP];
    {
        codec_zenoh_locator_t actual = {0};
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
            /* The text's own length: a row of a codec printed from its plan
             * states no length the encode derives (docs/adr/0081 §3). */
            actual.locator_len = _str_len;
        }
        uint8_t _encoded_buf[CODEC_ZENOH_LOCATOR_MAX_BYTES];
        size_t _encoded_len = 0;
        sce_forge_codec_status_t _enc_st = codec_zenoh_locator_encode_to_buf(&actual, _encoded_buf, sizeof(_encoded_buf), &_encoded_len);
        (void)_enc_st;
        static const uint8_t _expected[] = { 0x00 };
        if (_encoded_len != sizeof _expected
            || memcmp(_encoded_buf, _expected, _encoded_len) != 0) {
            sce_text_writer_t _report = sce_forge_test_vector_line(_line, sizeof _line, 39u);
            SCE_TEXT_PUT_LITERAL(&_report, "encode length=");
            sce_text_put_u64(&_report, (uint64_t)_encoded_len);
            SCE_TEXT_PUT_LITERAL(&_report, " (expected ");
            sce_text_put_u64(&_report, (uint64_t)sizeof _expected);
            sce_text_put_char(&_report, ')');
            sce_forge_test_vector_failed("codec_zenoh_locator", &_report);
            ++failures;
        }
        sce_forge_cursor_t _cursor = sce_forge_cursor_init(_expected, sizeof _expected);
        codec_zenoh_locator_t decoded = {0};
        sce_forge_codec_status_t _st = codec_zenoh_locator_decode(&_cursor, &decoded);
        if (_st != SCE_FORGE_CODEC_OK) {
            sce_text_writer_t _report = sce_forge_test_vector_line(_line, sizeof _line, 39u);
            SCE_TEXT_PUT_LITERAL(&_report, "decode status=");
            sce_text_put_i64(&_report, (int64_t)_st);
            sce_forge_test_vector_failed("codec_zenoh_locator", &_report);
            ++failures;
        } else if (sce_forge_cursor_remaining(&_cursor) != 0) {
            sce_text_writer_t _report = sce_forge_test_vector_line(_line, sizeof _line, 39u);
            SCE_TEXT_PUT_LITERAL(&_report, "decode left ");
            sce_text_put_u64(&_report, (uint64_t)sce_forge_cursor_remaining(&_cursor));
            SCE_TEXT_PUT_LITERAL(&_report, " bytes unconsumed");
            sce_forge_test_vector_failed("codec_zenoh_locator", &_report);
            ++failures;
        } else {
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
                    sce_text_writer_t _report = sce_forge_test_vector_line(_line, sizeof _line, 39u);
                    SCE_TEXT_PUT_LITERAL(&_report, "field `locator` (string) mismatch");
                    sce_forge_test_vector_failed("codec_zenoh_locator", &_report);
                    ++failures;
                }
            }
        }
    }
    {
        codec_zenoh_locator_t actual = {0};
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
            /* The text's own length: a row of a codec printed from its plan
             * states no length the encode derives (docs/adr/0081 §3). */
            actual.locator_len = _str_len;
        }
        uint8_t _encoded_buf[CODEC_ZENOH_LOCATOR_MAX_BYTES];
        size_t _encoded_len = 0;
        sce_forge_codec_status_t _enc_st = codec_zenoh_locator_encode_to_buf(&actual, _encoded_buf, sizeof(_encoded_buf), &_encoded_len);
        (void)_enc_st;
        static const uint8_t _expected[] = { 0x03, 0x61, 0x62, 0x63 };
        if (_encoded_len != sizeof _expected
            || memcmp(_encoded_buf, _expected, _encoded_len) != 0) {
            sce_text_writer_t _report = sce_forge_test_vector_line(_line, sizeof _line, 43u);
            SCE_TEXT_PUT_LITERAL(&_report, "encode length=");
            sce_text_put_u64(&_report, (uint64_t)_encoded_len);
            SCE_TEXT_PUT_LITERAL(&_report, " (expected ");
            sce_text_put_u64(&_report, (uint64_t)sizeof _expected);
            sce_text_put_char(&_report, ')');
            sce_forge_test_vector_failed("codec_zenoh_locator", &_report);
            ++failures;
        }
        sce_forge_cursor_t _cursor = sce_forge_cursor_init(_expected, sizeof _expected);
        codec_zenoh_locator_t decoded = {0};
        sce_forge_codec_status_t _st = codec_zenoh_locator_decode(&_cursor, &decoded);
        if (_st != SCE_FORGE_CODEC_OK) {
            sce_text_writer_t _report = sce_forge_test_vector_line(_line, sizeof _line, 43u);
            SCE_TEXT_PUT_LITERAL(&_report, "decode status=");
            sce_text_put_i64(&_report, (int64_t)_st);
            sce_forge_test_vector_failed("codec_zenoh_locator", &_report);
            ++failures;
        } else if (sce_forge_cursor_remaining(&_cursor) != 0) {
            sce_text_writer_t _report = sce_forge_test_vector_line(_line, sizeof _line, 43u);
            SCE_TEXT_PUT_LITERAL(&_report, "decode left ");
            sce_text_put_u64(&_report, (uint64_t)sce_forge_cursor_remaining(&_cursor));
            SCE_TEXT_PUT_LITERAL(&_report, " bytes unconsumed");
            sce_forge_test_vector_failed("codec_zenoh_locator", &_report);
            ++failures;
        } else {
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
                    sce_text_writer_t _report = sce_forge_test_vector_line(_line, sizeof _line, 43u);
                    SCE_TEXT_PUT_LITERAL(&_report, "field `locator` (string) mismatch");
                    sce_forge_test_vector_failed("codec_zenoh_locator", &_report);
                    ++failures;
                }
            }
        }
    }
    {
        codec_zenoh_locator_t actual = {0};
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
            /* The text's own length: a row of a codec printed from its plan
             * states no length the encode derives (docs/adr/0081 §3). */
            actual.locator_len = _str_len;
        }
        uint8_t _encoded_buf[CODEC_ZENOH_LOCATOR_MAX_BYTES];
        size_t _encoded_len = 0;
        sce_forge_codec_status_t _enc_st = codec_zenoh_locator_encode_to_buf(&actual, _encoded_buf, sizeof(_encoded_buf), &_encoded_len);
        (void)_enc_st;
        static const uint8_t _expected[] = { 0x12, 0x74, 0x63, 0x70, 0x2f, 0x31, 0x32, 0x37, 0x2e, 0x30, 0x2e, 0x30, 0x2e, 0x31, 0x3a, 0x37, 0x34, 0x34, 0x37 };
        if (_encoded_len != sizeof _expected
            || memcmp(_encoded_buf, _expected, _encoded_len) != 0) {
            sce_text_writer_t _report = sce_forge_test_vector_line(_line, sizeof _line, 47u);
            SCE_TEXT_PUT_LITERAL(&_report, "encode length=");
            sce_text_put_u64(&_report, (uint64_t)_encoded_len);
            SCE_TEXT_PUT_LITERAL(&_report, " (expected ");
            sce_text_put_u64(&_report, (uint64_t)sizeof _expected);
            sce_text_put_char(&_report, ')');
            sce_forge_test_vector_failed("codec_zenoh_locator", &_report);
            ++failures;
        }
        sce_forge_cursor_t _cursor = sce_forge_cursor_init(_expected, sizeof _expected);
        codec_zenoh_locator_t decoded = {0};
        sce_forge_codec_status_t _st = codec_zenoh_locator_decode(&_cursor, &decoded);
        if (_st != SCE_FORGE_CODEC_OK) {
            sce_text_writer_t _report = sce_forge_test_vector_line(_line, sizeof _line, 47u);
            SCE_TEXT_PUT_LITERAL(&_report, "decode status=");
            sce_text_put_i64(&_report, (int64_t)_st);
            sce_forge_test_vector_failed("codec_zenoh_locator", &_report);
            ++failures;
        } else if (sce_forge_cursor_remaining(&_cursor) != 0) {
            sce_text_writer_t _report = sce_forge_test_vector_line(_line, sizeof _line, 47u);
            SCE_TEXT_PUT_LITERAL(&_report, "decode left ");
            sce_text_put_u64(&_report, (uint64_t)sce_forge_cursor_remaining(&_cursor));
            SCE_TEXT_PUT_LITERAL(&_report, " bytes unconsumed");
            sce_forge_test_vector_failed("codec_zenoh_locator", &_report);
            ++failures;
        } else {
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
                    sce_text_writer_t _report = sce_forge_test_vector_line(_line, sizeof _line, 47u);
                    SCE_TEXT_PUT_LITERAL(&_report, "field `locator` (string) mismatch");
                    sce_forge_test_vector_failed("codec_zenoh_locator", &_report);
                    ++failures;
                }
            }
        }
    }
    {
        codec_zenoh_locator_t actual = {0};
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
            /* The text's own length: a row of a codec printed from its plan
             * states no length the encode derives (docs/adr/0081 §3). */
            actual.locator_len = _str_len;
        }
        uint8_t _encoded_buf[CODEC_ZENOH_LOCATOR_MAX_BYTES];
        size_t _encoded_len = 0;
        sce_forge_codec_status_t _enc_st = codec_zenoh_locator_encode_to_buf(&actual, _encoded_buf, sizeof(_encoded_buf), &_encoded_len);
        (void)_enc_st;
        static const uint8_t _expected[] = { 0x06, 0x68, 0xc3, 0xa9, 0x6c, 0x6c, 0x6f };
        if (_encoded_len != sizeof _expected
            || memcmp(_encoded_buf, _expected, _encoded_len) != 0) {
            sce_text_writer_t _report = sce_forge_test_vector_line(_line, sizeof _line, 51u);
            SCE_TEXT_PUT_LITERAL(&_report, "encode length=");
            sce_text_put_u64(&_report, (uint64_t)_encoded_len);
            SCE_TEXT_PUT_LITERAL(&_report, " (expected ");
            sce_text_put_u64(&_report, (uint64_t)sizeof _expected);
            sce_text_put_char(&_report, ')');
            sce_forge_test_vector_failed("codec_zenoh_locator", &_report);
            ++failures;
        }
        sce_forge_cursor_t _cursor = sce_forge_cursor_init(_expected, sizeof _expected);
        codec_zenoh_locator_t decoded = {0};
        sce_forge_codec_status_t _st = codec_zenoh_locator_decode(&_cursor, &decoded);
        if (_st != SCE_FORGE_CODEC_OK) {
            sce_text_writer_t _report = sce_forge_test_vector_line(_line, sizeof _line, 51u);
            SCE_TEXT_PUT_LITERAL(&_report, "decode status=");
            sce_text_put_i64(&_report, (int64_t)_st);
            sce_forge_test_vector_failed("codec_zenoh_locator", &_report);
            ++failures;
        } else if (sce_forge_cursor_remaining(&_cursor) != 0) {
            sce_text_writer_t _report = sce_forge_test_vector_line(_line, sizeof _line, 51u);
            SCE_TEXT_PUT_LITERAL(&_report, "decode left ");
            sce_text_put_u64(&_report, (uint64_t)sce_forge_cursor_remaining(&_cursor));
            SCE_TEXT_PUT_LITERAL(&_report, " bytes unconsumed");
            sce_forge_test_vector_failed("codec_zenoh_locator", &_report);
            ++failures;
        } else {
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
                    sce_text_writer_t _report = sce_forge_test_vector_line(_line, sizeof _line, 51u);
                    SCE_TEXT_PUT_LITERAL(&_report, "field `locator` (string) mismatch");
                    sce_forge_test_vector_failed("codec_zenoh_locator", &_report);
                    ++failures;
                }
            }
        }
    }
    return failures;
}

#endif  /* SCE_FORGE_CODEC_ZENOH_LOCATOR_TEST_H */
