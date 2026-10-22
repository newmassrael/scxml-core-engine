// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The writer and the reader a saved state's JSON text goes through on the C11
// backend (docs/adr/0045, section 7): `sce/saved_json.h` and `sce/saved_values.h`.
// The C half of `tests/saved/SavedStateTest.cpp`, held to the same cases:
//
//   * every shared saved text (`sce-build/tests/fixtures/static_datamodel/saved/`)
//     reads and is written back as it was, byte for byte — a number in the spelling it
//     was written in, an object in the order of its members;
//   * a text that is not one JSON value is refused in the words every backend uses;
//   * every value type is written as its saved form and read back as it was, and a
//     value of another kind or another width is refused, not converted;
//   * a text, a byte string and a list are held to the bounds the machine has.
//
// Nothing here allocates: the writer writes into a buffer on the stack and the reader
// walks the text it was handed, which is the contract the MCU profile needs.

// `opendir` is POSIX, and the build asks for strict C11.
#define _POSIX_C_SOURCE 200809L

#include <sce/saved_json.h>
#include <sce/saved_values.h>

#include <dirent.h>
#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#ifndef SCE_SAVED_FIXTURES_DIR
#error "SCE_SAVED_FIXTURES_DIR names sce-build/tests/fixtures/static_datamodel/saved; the build defines it"
#endif

static int failures = 0;

#define CHECK(condition, ...)                                                                                          \
    do {                                                                                                               \
        if (!(condition)) {                                                                                            \
            (void)fprintf(stderr, "FAIL %s:%d: ", __FILE__, __LINE__);                                                 \
            (void)fprintf(stderr, __VA_ARGS__);                                                                        \
            (void)fputc('\n', stderr);                                                                                 \
            ++failures;                                                                                                \
        }                                                                                                              \
    } while (0)

enum { TEXT_CAP = 16384 };

// The text of `path` without its closing newline; false when it cannot be read.
static bool read_trimmed(const char *path, char *out, size_t cap, size_t *len) {
    FILE *file = fopen(path, "rb");
    if (file == NULL) {
        return false;
    }
    const size_t n = fread(out, 1u, cap - 1u, file);
    (void)fclose(file);
    size_t end = n;
    while (end > 0u && (out[end - 1u] == '\n' || out[end - 1u] == '\r')) {
        --end;
    }
    out[end] = '\0';
    *len = end;
    return true;
}

// `text` read and written again.
static bool rewritten(const char *text, size_t len, char *out, size_t cap, size_t *out_len, sce_saved_error_t *err) {
    sce_jv_t value;
    sce_jw_t writer;
    if (!sce_json_parse(text, len, &value, err)) {
        return false;
    }
    sce_jw_init(&writer, out, cap);
    sce_jw_value(&writer, value);
    *out_len = writer.len;
    return sce_jw_finish(&writer);
}

static int check_directory(const char *directory) {
    DIR *dir = opendir(directory);
    int seen = 0;
    if (dir == NULL) {
        CHECK(false, "cannot open %s", directory);
        return 0;
    }
    for (struct dirent *entry = readdir(dir); entry != NULL; entry = readdir(dir)) {
        const size_t n = strlen(entry->d_name);
        if (n < 6u || strcmp(entry->d_name + n - 5u, ".json") != 0) {
            continue;
        }
        char path[1024];
        static char text[TEXT_CAP];
        static char again[TEXT_CAP];
        size_t len = 0u;
        size_t again_len = 0u;
        sce_saved_error_t err;
        (void)snprintf(path, sizeof(path), "%s/%s", directory, entry->d_name);
        if (!read_trimmed(path, text, sizeof(text), &len)) {
            CHECK(false, "cannot read %s", path);
            continue;
        }
        const bool ok = rewritten(text, len, again, sizeof(again), &again_len, &err);
        CHECK(ok, "%s: %s", entry->d_name, ok ? "" : err.message);
        if (ok) {
            CHECK(again_len == len && memcmp(text, again, len) == 0, "%s was not written back as it was",
                  entry->d_name);
        }
        ++seen;
    }
    (void)closedir(dir);
    return seen;
}

static void every_shared_text_reads_and_writes_back_as_it_was(void) {
    char format1[1024];
    (void)snprintf(format1, sizeof(format1), "%s/format1", SCE_SAVED_FIXTURES_DIR);
    CHECK(check_directory(SCE_SAVED_FIXTURES_DIR) >= 8, "the shared format-2 texts lost instances");
    CHECK(check_directory(format1) >= 10, "the shared format-1 texts lost instances");
}

static void a_number_keeps_the_spelling_it_was_written_in(void) {
    const char *text = "{\"a\":[1,-0,1.50,1E+2,9223372036854775808],\"b\":null,\"c\":\"\\u00e9\\ud83d\\ude00\"}";
    // A character written as a \u escape is written as the character.
    const char *want = "{\"a\":[1,-0,1.50,1E+2,9223372036854775808],\"b\":null,\"c\":\"\xC3\xA9\xF0\x9F\x98\x80\"}";
    char out[256];
    size_t n = 0u;
    sce_saved_error_t err;
    CHECK(rewritten(text, strlen(text), out, sizeof(out), &n, &err), "%s", err.message);
    CHECK(strcmp(out, want) == 0, "wrote %s", out);
}

static void an_object_keeps_the_order_its_members_were_written_in(void) {
    const char *text = "{\"z\":1,\"a\":2,\"m\":3}";
    sce_jv_t value;
    sce_jv_t key;
    sce_jv_t member;
    sce_jiter_t it;
    char order[8] = {0};
    size_t n = 0u;
    sce_saved_error_t err;
    CHECK(sce_json_parse(text, strlen(text), &value, &err), "%s", err.message);
    CHECK(sce_jv_iter(value, &it), "an object iterates");
    while (sce_jiter_member(&it, &key, &member) && n < 4u) {
        order[n++] = *(key.p + 1);
    }
    CHECK(strcmp(order, "zam") == 0, "order %s", order);
}

static void a_text_that_is_not_one_json_value_is_refused(void) {
    static const char *const cases[][2] = {
        {"{\"a\":1,\"a\":2}", "not JSON (the field \"a\" appears twice)"},
        {"1 2", "more than one JSON value"},
        {"[1,]", "not JSON (unexpected ']')"},
        {"\"\\q\"", "not JSON (unknown escape '\\q')"},
        {"\"\\u12\"", "not JSON (a \\u escape is short)"},
        {"\"\\u12zz\"", "not JSON (a \\u escape is not hex)"},
        {"-", "not JSON (a number has no digits)"},
        {"{\"a\" 1}", "not JSON (a ':' was expected)"},
        {"tru", "not JSON (expected 'true')"},
        {"", "not JSON (it ends where a value was expected)"},
        {"[1", "not JSON (it ends unclosed)"},
        {"\"open", "not JSON (a text ends unclosed)"},
        {"{1:2}", "not JSON (a field name was expected)"},
    };
    for (size_t i = 0u; i < sizeof(cases) / sizeof(cases[0]); ++i) {
        sce_jv_t value;
        sce_saved_error_t err;
        err.message[0] = '\0';
        CHECK(!sce_json_parse(cases[i][0], strlen(cases[i][0]), &value, &err), "%s was read", cases[i][0]);
        CHECK(strcmp(err.message, cases[i][1]) == 0, "%s: refused as '%s', wanted '%s'", cases[i][0], err.message,
              cases[i][1]);
    }
}

static void a_lone_surrogate_reads_as_the_replacement_character(void) {
    const char *text = "\"\\ud800x\\udc00\"";
    sce_jv_t value;
    char out[16];
    sce_saved_error_t err;
    CHECK(sce_json_parse(text, strlen(text), &value, &err), "%s", err.message);
    CHECK(sce_jv_string(value, out, sizeof(out), NULL), "fits");
    CHECK(strcmp(out, "\xEF\xBF\xBD"
                      "x"
                      "\xEF\xBF\xBD") == 0,
          "read %s", out);
}

// `write` writes into a writer; the text it wrote.
typedef void (*write_fn)(sce_jw_t *w);

static const char *written(write_fn write, char *out, size_t cap) {
    sce_jw_t w;
    sce_jw_init(&w, out, cap);
    write(&w);
    (void)sce_jw_finish(&w);
    return out;
}

static void w_uint8(sce_jw_t *w) {
    sce_saved_write_uint8(w, 200u);
}

static void w_int64_min(sce_jw_t *w) {
    sce_saved_write_int64(w, INT64_MIN);
}

static void w_uint64_max(sce_jw_t *w) {
    sce_saved_write_uint64(w, UINT64_MAX);
}

static void w_float32(sce_jw_t *w) {
    sce_saved_write_float32(w, 0.3F);
}

static void w_five(sce_jw_t *w) {
    sce_saved_write_float64(w, 5.0);
}

static void w_big(sce_jw_t *w) {
    sce_saved_write_float64(w, 1e21);
}

static void w_nan(sce_jw_t *w) {
    sce_saved_write_float64(w, (double)NAN);
}

static void w_bytes(sce_jw_t *w) {
    static const uint8_t bytes[] = {0x00u, 0xE9u, 0xFFu};
    sce_saved_write_bytes(w, bytes, sizeof(bytes));
}

static void every_value_type_is_written_as_its_saved_form_and_read_back_as_it_was(void) {
    char out[64];
    CHECK(strcmp(written(w_uint8, out, sizeof(out)), "200") == 0, "uint8 %s", out);
    CHECK(strcmp(written(w_int64_min, out, sizeof(out)), "\"-9223372036854775808\"") == 0, "int64 %s", out);
    CHECK(strcmp(written(w_uint64_max, out, sizeof(out)), "\"18446744073709551615\"") == 0, "uint64 %s", out);
    CHECK(strcmp(written(w_float32, out, sizeof(out)), "0.30000001192092896") == 0, "float32 %s", out);
    CHECK(strcmp(written(w_five, out, sizeof(out)), "5") == 0, "float64 %s", out);
    CHECK(strcmp(written(w_big, out, sizeof(out)), "1e+21") == 0, "float64 big %s", out);
    CHECK(strcmp(written(w_nan, out, sizeof(out)), "\"NaN\"") == 0, "nan %s", out);
    CHECK(strcmp(written(w_bytes, out, sizeof(out)), "\"\\u0000\xC3\xA9\xC3\xBF\"") == 0, "bytes %s", out);

    sce_jv_t v;
    sce_saved_error_t err;
    uint8_t u8 = 0u;
    int8_t i8 = 0;
    uint32_t u32 = 0u;
    int64_t i64 = 0;
    uint64_t u64 = 0u;
    double f64 = 0.0;
    float f32 = 0.0F;
    bool flag = false;
#define PARSE(text) CHECK(sce_json_parse(text, strlen(text), &v, &err), "%s", err.message)
    PARSE("255");
    CHECK(sce_saved_read_uint8(v, "x", &u8, &err) && u8 == 255u, "uint8");
    PARSE("-128");
    CHECK(sce_saved_read_int8(v, "x", &i8, &err) && i8 == -128, "int8");
    PARSE("4294967295");
    CHECK(sce_saved_read_uint32(v, "x", &u32, &err) && u32 == 4294967295U, "uint32");
    PARSE("\"-9223372036854775808\"");
    CHECK(sce_saved_read_int64(v, "x", &i64, &err) && i64 == INT64_MIN, "int64");
    PARSE("\"18446744073709551615\"");
    CHECK(sce_saved_read_uint64(v, "x", &u64, &err) && u64 == UINT64_MAX, "uint64");
    PARSE("1.5");
    CHECK(sce_saved_read_float64(v, "x", &f64, &err) && f64 == 1.5, "float64");
    PARSE("\"-Infinity\"");
    CHECK(sce_saved_read_float32(v, "x", &f32, &err) && isinf(f32), "float32 infinity");
    PARSE("1e999");
    CHECK(sce_saved_read_float64(v, "x", &f64, &err) && isinf(f64), "float64 infinity");
    PARSE("true");
    CHECK(sce_saved_read_bool(v, "x", &flag, &err) && flag, "bool");
    // A 32-bit real is rounded to the single it is.
    PARSE("0.3");
    CHECK(sce_saved_read_float32(v, "x", &f32, &err) && f32 == 0.3F, "float32 rounds");
#undef PARSE
}

static const char *refused_u8(const char *text, char *message) {
    sce_jv_t v;
    sce_saved_error_t err;
    uint8_t out = 0u;
    message[0] = '\0';
    if (sce_json_parse(text, strlen(text), &v, &err) && !sce_saved_read_uint8(v, "x", &out, &err)) {
        (void)snprintf(message, SCE_SAVED_ERROR_LEN, "%s", err.message);
    }
    return message;
}

static void a_value_of_another_kind_or_another_width_is_refused_not_converted(void) {
    char m[SCE_SAVED_ERROR_LEN];
    CHECK(strcmp(refused_u8("256", m), "'x' (256) is not a whole number its type can hold") == 0, "%s", m);
    CHECK(strcmp(refused_u8("-1", m), "'x' (-1) is not a whole number its type can hold") == 0, "%s", m);
    CHECK(strcmp(refused_u8("1.5", m), "'x' (1.5) is not a whole number its type can hold") == 0, "%s", m);
    CHECK(strcmp(refused_u8("\"1\"", m), "'x' is not a number") == 0, "%s", m);
    CHECK(strcmp(refused_u8("null", m), "'x' is absent (null), and its type is not optional") == 0, "%s", m);

    sce_jv_t v;
    sce_saved_error_t err;
    int64_t wide = 0;
    char text[8];
    bool flag = false;
    CHECK(sce_json_parse("5", 1u, &v, &err), "%s", err.message);
    CHECK(!sce_saved_read_int64(v, "x", &wide, &err) &&
              strcmp(err.message, "'x' is not a 64-bit whole number written as a text") == 0,
          "%s", err.message);
    CHECK(sce_json_parse("\"9223372036854775808\"", 21u, &v, &err), "%s", err.message);
    CHECK(!sce_saved_read_int64(v, "x", &wide, &err) &&
              strcmp(err.message, "'x' (9223372036854775808) is not a whole number its type can hold") == 0,
          "%s", err.message);
    CHECK(sce_json_parse("1", 1u, &v, &err), "%s", err.message);
    CHECK(!sce_saved_read_string_bounded(v, "x", text, 7u, &err) && strcmp(err.message, "'x' is not a text") == 0, "%s",
          err.message);
    CHECK(sce_json_parse("\"true\"", 6u, &v, &err), "%s", err.message);
    CHECK(!sce_saved_read_bool(v, "x", &flag, &err) && strcmp(err.message, "'x' is not a truth value") == 0, "%s",
          err.message);
}

static void a_text_a_byte_string_and_a_list_are_held_to_the_bounds_the_machine_has(void) {
    sce_jv_t v;
    sce_saved_error_t err;
    char text[16];
    uint8_t bytes[8];
    size_t n = 0u;
    CHECK(sce_json_parse("\"h\xC3\xA9llo\"", 8u, &v, &err), "%s", err.message);
    CHECK(!sce_saved_read_string_bounded(v, "t", text, 5u, &err) &&
              strcmp(err.message, "'t' holds 6 UTF-8 bytes, past the 5 it is bounded by") == 0,
          "%s", err.message);
    CHECK(sce_saved_read_string_bounded(v, "t", text, 6u, &err) && strcmp(text, "h\xC3\xA9llo") == 0, "fits");

    CHECK(sce_json_parse("\"a\xC4\x80\"", 5u, &v, &err), "%s", err.message);
    CHECK(!sce_saved_read_bytes(v, "b", bytes, 8u, &n, &err) &&
              strcmp(err.message, "'b' holds U+0100, which is past U+00FF and so no byte") == 0,
          "%s", err.message);
    CHECK(sce_json_parse("\"abc\"", 5u, &v, &err), "%s", err.message);
    CHECK(!sce_saved_read_bytes(v, "b", bytes, 2u, &n, &err) &&
              strcmp(err.message, "'b' holds 3 bytes, past the 2 it is bounded by") == 0,
          "%s", err.message);
    CHECK(sce_json_parse("\"\\u0000\xC3\xA9\xC3\xBF\"", 12u, &v, &err), "%s", err.message);
    CHECK(sce_saved_read_bytes(v, "b", bytes, 8u, &n, &err) && n == 3u && bytes[0] == 0u && bytes[1] == 0xE9u &&
              bytes[2] == 0xFFu,
          "a byte string is read back from its Latin-1 text");

    size_t count = 0u;
    CHECK(sce_json_parse("[1,2,300]", 9u, &v, &err), "%s", err.message);
    CHECK(sce_saved_list(v, "l", 8u, &count, &err) && count == 3u, "a list within its bound");
    CHECK(!sce_saved_list(v, "l", 2u, &count, &err) &&
              strcmp(err.message, "'l' holds 3 elements, past the 2 it is bounded by") == 0,
          "%s", err.message);
}

static void a_session_id_is_an_xml_nmtoken(void) {
    sce_saved_error_t err;
    CHECK(sce_saved_session_id_ok("fixture-root", &err), "%s", err.message);
    CHECK(sce_saved_session_id_ok("fixture-root.working.1.worker", &err), "%s", err.message);
    CHECK(!sce_saved_session_id_ok("", &err), "an empty id");
    CHECK(!sce_saved_session_id_ok("a b", &err) && strstr(err.message, "U+0020") != NULL, "%s", err.message);
    CHECK(!sce_saved_session_id_ok("a\xFF", &err), "a malformed byte is no name character");
}

static void a_variant_is_read_by_the_name_its_document_declares(void) {
    static const char *const names[] = {"low", "high"};
    sce_jv_t v;
    sce_saved_error_t err;
    size_t index = 9u;
    CHECK(sce_json_parse("\"high\"", 6u, &v, &err), "%s", err.message);
    CHECK(sce_saved_read_variant(v, "level", "Level", names, 2u, &index, &err) && index == 1u, "high");
    CHECK(sce_json_parse("\"mid\"", 5u, &v, &err), "%s", err.message);
    CHECK(!sce_saved_read_variant(v, "level", "Level", names, 2u, &index, &err) &&
              strcmp(err.message, "'level' (mid) is not a variant of Level") == 0,
          "%s", err.message);
}

static void a_writer_over_no_buffer_measures_the_text(void) {
    sce_jw_t measure;
    sce_jw_t real;
    char out[64];
    sce_jw_init(&measure, NULL, 0u);
    sce_jw_begin_object(&measure);
    sce_jw_key(&measure, "a");
    sce_jw_int(&measure, 12);
    sce_jw_key(&measure, "b");
    sce_jw_begin_array(&measure);
    sce_jw_element(&measure);
    sce_jw_bool(&measure, true);
    sce_jw_element(&measure);
    sce_jw_null(&measure);
    sce_jw_end_array(&measure);
    sce_jw_end_object(&measure);
    CHECK(!sce_jw_finish(&measure), "no buffer holds nothing");

    sce_jw_init(&real, out, sizeof(out));
    sce_jw_begin_object(&real);
    sce_jw_key(&real, "a");
    sce_jw_int(&real, 12);
    sce_jw_key(&real, "b");
    sce_jw_begin_array(&real);
    sce_jw_element(&real);
    sce_jw_bool(&real, true);
    sce_jw_element(&real);
    sce_jw_null(&real);
    sce_jw_end_array(&real);
    sce_jw_end_object(&real);
    CHECK(sce_jw_finish(&real) && strcmp(out, "{\"a\":12,\"b\":[true,null]}") == 0, "wrote %s", out);
    CHECK(measure.len == real.len, "measured %zu, wrote %zu", measure.len, real.len);

    // A buffer too small says so, and the length says how large to make it.
    sce_jw_init(&real, out, 5u);
    sce_jw_string(&real, "abcdefgh");
    CHECK(!sce_jw_finish(&real) && real.len == 10u, "len %zu", real.len);
}

int main(void) {
    every_shared_text_reads_and_writes_back_as_it_was();
    a_number_keeps_the_spelling_it_was_written_in();
    an_object_keeps_the_order_its_members_were_written_in();
    a_text_that_is_not_one_json_value_is_refused();
    a_lone_surrogate_reads_as_the_replacement_character();
    every_value_type_is_written_as_its_saved_form_and_read_back_as_it_was();
    a_value_of_another_kind_or_another_width_is_refused_not_converted();
    a_text_a_byte_string_and_a_list_are_held_to_the_bounds_the_machine_has();
    a_session_id_is_an_xml_nmtoken();
    a_variant_is_read_by_the_name_its_document_declares();
    a_writer_over_no_buffer_measures_the_text();
    if (failures != 0) {
        (void)fprintf(stderr, "%d check(s) failed\n", failures);
        return 1;
    }
    (void)puts("saved json: ok");
    return 0;
}
