/* SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael */

/*
 * sce/forge/content_line.h — the content lines a `sce:encoding="content-line"`
 * codec reads and writes (SCE_FORGE.md §4.6.4). The same properties the Rust
 * runtime's own tests pin (backends/rust/forge-runtime/src/content_line.rs):
 * unfolding wherever the sender cut, names read without regard to case, a
 * nested component and an undeclared property skipped whole, truncation told
 * from malformation, TEXT escapes, a value held to its type, and a writer that
 * folds before the unit that would pass 75 octets and refuses a value a line
 * could not carry. What a generated codec does with these is the conformance
 * harness's (codec_content_line_event), which this file does not repeat.
 */

#include "sce/forge/content_line.h"

#include <stdio.h>
#include <string.h>

static int failures = 0;

#define CHECK(cond, what)                                                                                              \
    do {                                                                                                               \
        if (!(cond)) {                                                                                                 \
            ++failures;                                                                                                \
            fprintf(stderr, "FAIL %s:%d %s\n", __FILE__, __LINE__, what);                                              \
        }                                                                                                              \
    } while (0)

/* What a read of a VEVENT's UID and SUMMARY found: up to four (name, value)
 * pairs, or the status that refused it. */
typedef struct {
    char name[4][8];
    char value[4][40];
    size_t count;
    sce_forge_codec_status_t status;
} outcome_t;

static outcome_t read_all(const char *text, size_t len) {
    outcome_t out;
    sce_forge_cl_reader_t reader;
    memset(&out, 0, sizeof out);
    out.status = sce_forge_cl_reader_begin(&reader, (const uint8_t *)text, len, "VEVENT");
    while (out.status == SCE_FORGE_CODEC_OK) {
        sce_forge_cl_property_t line;
        bool has = false;
        const char *name = NULL;
        size_t n = 0;
        out.status = sce_forge_cl_reader_next(&reader, &line, &has);
        if (out.status != SCE_FORGE_CODEC_OK || !has) {
            break;
        }
        if (sce_forge_cl_property_is(&line, "UID")) {
            name = "UID";
        } else if (sce_forge_cl_property_is(&line, "SUMMARY")) {
            name = "SUMMARY";
        } else {
            continue;
        }
        out.status =
            sce_forge_cl_property_read_string(&line, 32, strcmp(name, "SUMMARY") == 0, out.value[out.count], &n);
        if (out.status != SCE_FORGE_CODEC_OK) {
            break;
        }
        out.value[out.count][n] = '\0';
        strcpy(out.name[out.count], name);
        out.count++;
    }
    return out;
}

/* `SUMMARY:<value>` inside a VEVENT. */
static outcome_t one(const char *value, size_t value_len) {
    char text[512];
    static const char head[] = "BEGIN:VEVENT\r\nSUMMARY:";
    static const char tail[] = "\r\nEND:VEVENT\r\n";
    size_t n = 0;
    memcpy(text, head, sizeof head - 1U);
    n += sizeof head - 1U;
    memcpy(text + n, value, value_len);
    n += value_len;
    memcpy(text + n, tail, sizeof tail - 1U);
    n += sizeof tail - 1U;
    return read_all(text, n);
}

#define ONE(s) one(s, sizeof(s) - 1U)
#define READ(s) read_all(s, sizeof(s) - 1U)

static bool reads(const outcome_t *o, const char *name, const char *value) {
    return o->status == SCE_FORGE_CODEC_OK && o->count == 1U && strcmp(o->name[0], name) == 0 &&
           strcmp(o->value[0], value) == 0;
}

static void a_component_is_read_property_by_property_and_stops_after_its_end(void) {
    static const char text[] =
        "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VEVENT\r\nUID:a1\r\nSUMMARY:hi\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
    sce_forge_cl_reader_t reader;
    int seen = 0;
    CHECK(sce_forge_cl_reader_begin(&reader, (const uint8_t *)text, sizeof text - 1U, "VEVENT") == SCE_FORGE_CODEC_OK,
          "the component was not found");
    for (;;) {
        sce_forge_cl_property_t line;
        bool has = false;
        CHECK(sce_forge_cl_reader_next(&reader, &line, &has) == SCE_FORGE_CODEC_OK, "next");
        if (!has) {
            break;
        }
        CHECK(sce_forge_cl_property_is(&line, "UID") || sce_forge_cl_property_is(&line, "SUMMARY"),
              "a property nobody declared surfaced");
        ++seen;
    }
    CHECK(seen == 2, "two properties");
    CHECK(strcmp(text + sce_forge_cl_reader_consumed(&reader), "END:VCALENDAR\r\n") == 0,
          "the walk did not stop after END:VEVENT");
}

static void names_are_matched_without_regard_to_case_and_lf_ends_a_line(void) {
    const outcome_t o = READ("begin:vevent\nuid:x\nend:Vevent\n");
    CHECK(reads(&o, "UID", "x"), "a lower-case LF component");
}

static void a_fold_is_removed_wherever_the_sender_cut(void) {
    outcome_t o = READ("BEGIN:VEVENT\r\nUI\r\n D:ab\r\n\tcd\r\n e\r\nEND:VEVENT\r\n");
    CHECK(reads(&o, "UID", "abcde"), "a fold inside a name and inside a value");
    o = ONE("\xc3\r\n \xa9");
    CHECK(reads(&o, "SUMMARY", "\xc3\xa9"), "a cut inside a UTF-8 character");
}

static void a_nested_component_and_an_undeclared_property_are_skipped_whole(void) {
    const outcome_t o = READ("BEGIN:VEVENT\r\nX-NOISE;A=\"b:c\":d\r\nBEGIN:VALARM\r\nUID:inner\r\nBEGIN:X\r\nEND:X\r\n"
                             "END:VALARM\r\nUID:outer\r\nEND:VEVENT\r\n");
    CHECK(reads(&o, "UID", "outer"), "the nested component's own UID surfaced");
}

static void an_input_that_ends_early_needs_more_bytes(void) {
    static const char *const inputs[] = {"",
                                         "BEGIN:VCALENDAR\r\n",
                                         "BEGIN:VEVENT",
                                         "BEGIN:VEVENT\r\nUID:a",
                                         "BEGIN:VEVENT\r\nUID:a\r\n",
                                         "BEGIN:VEVENT\r\nUID:a\r\nEND:VEVEN",
                                         "BEGIN:VEVENT\r\nBEGIN:VALARM\r\nEND:VALARM\r\n"};
    size_t i;
    for (i = 0; i < sizeof inputs / sizeof inputs[0]; ++i) {
        const outcome_t o = read_all(inputs[i], strlen(inputs[i]));
        CHECK(o.status == SCE_FORGE_CODEC_NEED_MORE_BYTES, inputs[i]);
    }
    CHECK(READ("BEGIN:VEVENT\r\nUID:a\r\nEND:VEVENT").status == SCE_FORGE_CODEC_OK,
          "the END line needs no line break after it");
}

static void a_line_the_grammar_does_not_admit_is_malformed(void) {
    static const char *const inputs[] = {"BEGIN:VEVENT\r\n\r\nEND:VEVENT\r\n",
                                         "BEGIN:VEVENT\r\nUID\r\nEND:VEVENT\r\n",
                                         "BEGIN:VEVENT\r\n=x:y\r\nEND:VEVENT\r\n",
                                         "BEGIN:VEVENT\r\nUID:a\r\nEND:VTODO\r\n",
                                         "BEGIN:VEVENT\r\nUID;TZID:a\r\nEND:VEVENT\r\n",
                                         "BEGIN:VEVENT\r\nUID;TZID=\"a:b\r\nEND:VEVENT\r\n",
                                         "BEGIN:VEVENT\r\nUID;TZID=\"a\"b:c\r\nEND:VEVENT\r\n",
                                         "BEGIN:VEVENT\r\nUID;TZID=a\"b:c\r\nEND:VEVENT\r\n"};
    size_t i;
    for (i = 0; i < sizeof inputs / sizeof inputs[0]; ++i) {
        const outcome_t o = read_all(inputs[i], strlen(inputs[i]));
        CHECK(o.status == SCE_FORGE_CODEC_LINE_MALFORMED, inputs[i]);
    }
}

static void a_text_unescapes_and_a_bad_escape_is_refused(void) {
    static const char text[] = "BEGIN:VEVENT\r\nUID:a\\nb\r\nEND:VEVENT\r\n";
    sce_forge_cl_reader_t reader;
    sce_forge_cl_property_t line;
    bool has = false;
    char buf[8];
    size_t n = 0;
    outcome_t o = ONE("a\\\\b\\;c\\,d\\ne\\Nf;g,h");
    CHECK(reads(&o, "SUMMARY", "a\\b;c,d\ne\nf;g,h"), "every TEXT escape");
    o = ONE("a\\xb");
    CHECK(o.status == SCE_FORGE_CODEC_LINE_BAD_ESCAPE, "an unknown escape");
    o = ONE("a\\");
    CHECK(o.status == SCE_FORGE_CODEC_LINE_BAD_ESCAPE, "a backslash that ends the value");
    /* Without sce:value="text" a backslash is a byte like another. */
    (void)sce_forge_cl_reader_begin(&reader, (const uint8_t *)text, sizeof text - 1U, "VEVENT");
    (void)sce_forge_cl_reader_next(&reader, &line, &has);
    CHECK(sce_forge_cl_property_read_string(&line, 8, false, buf, &n) == SCE_FORGE_CODEC_OK && n == 4U &&
              memcmp(buf, "a\\nb", 4) == 0,
          "a backslash in a value that is not a TEXT");
}

static void a_control_character_bad_utf8_and_a_long_value_are_refused(void) {
    char thirty_three[33];
    char thirty_two[32];
    char escaped[64];
    size_t i;
    CHECK(ONE("a\x01"
              "b")
                  .status == SCE_FORGE_CODEC_LINE_BAD_VALUE,
          "a control character");
    CHECK(ONE("a\rb").status == SCE_FORGE_CODEC_LINE_BAD_VALUE, "a carriage return");
    CHECK(ONE("a\x7f"
              "b")
                  .status == SCE_FORGE_CODEC_LINE_BAD_VALUE,
          "a delete character");
    CHECK(ONE("a\xff"
              "b")
                  .status == SCE_FORGE_CODEC_LINE_BAD_VALUE,
          "bytes that are not UTF-8");
    CHECK(ONE("a\tb").status == SCE_FORGE_CODEC_OK, "a tab is a character like another");
    memset(thirty_three, 'x', sizeof thirty_three);
    memset(thirty_two, 'x', sizeof thirty_two);
    CHECK(one(thirty_three, sizeof thirty_three).status == SCE_FORGE_CODEC_LINE_TOO_LONG, "33 octets past 32");
    CHECK(one(thirty_two, sizeof thirty_two).status == SCE_FORGE_CODEC_OK, "32 octets in 32");
    for (i = 0; i < 32U; ++i) {
        escaped[2U * i] = '\\';
        escaped[2U * i + 1U] = ';';
    }
    CHECK(one(escaped, sizeof escaped).status == SCE_FORGE_CODEC_OK, "the size is that of the unescaped text");
}

static void parameters_are_read_by_name_quoted_or_not_and_the_rest_are_skipped(void) {
    static const char text[] =
        "BEGIN:VEVENT\r\nDTSTART;X-A=1;TZID=\"Europe/Seoul:KST\";X-B=\"p,q\",r:20260101T000000\r\nEND:VEVENT\r\n";
    sce_forge_cl_reader_t reader;
    sce_forge_cl_property_t line;
    bool has = false;
    char tzid[32];
    size_t tzid_len = 0;
    char value[32];
    size_t value_len = 0;
    bool got_tzid = false;
    (void)sce_forge_cl_reader_begin(&reader, (const uint8_t *)text, sizeof text - 1U, "VEVENT");
    (void)sce_forge_cl_reader_next(&reader, &line, &has);
    CHECK(has && sce_forge_cl_property_is(&line, "dtstart"), "the property");
    for (;;) {
        bool more = false;
        const sce_forge_codec_status_t s = sce_forge_cl_property_next_param(&line, &more);
        CHECK(s == SCE_FORGE_CODEC_OK, "a parameter was refused");
        if (s != SCE_FORGE_CODEC_OK || !more) {
            break;
        }
        if (sce_forge_cl_property_param_is(&line, "tzid")) {
            got_tzid =
                sce_forge_cl_property_read_param_string(&line, sizeof tzid, tzid, &tzid_len) == SCE_FORGE_CODEC_OK;
        }
    }
    CHECK(got_tzid && tzid_len == 16U && memcmp(tzid, "Europe/Seoul:KST", 16) == 0, "the quoted parameter");
    CHECK(sce_forge_cl_property_read_string(&line, sizeof value, false, value, &value_len) == SCE_FORGE_CODEC_OK &&
              value_len == 15U && memcmp(value, "20260101T000000", 15) == 0,
          "the value after the parameters");
    has = true;
    CHECK(sce_forge_cl_reader_next(&reader, &line, &has) == SCE_FORGE_CODEC_OK && !has, "the component ends after it");
}

static void a_parameter_of_two_values_or_past_its_bound_is_refused_where_it_is_read(void) {
    static const struct {
        const char *text;
        sce_forge_codec_status_t want;
    } cases[] = {
        {"BEGIN:VEVENT\r\nDTSTART;TZID=a,b:x\r\nEND:VEVENT\r\n", SCE_FORGE_CODEC_LINE_BAD_VALUE},
        {"BEGIN:VEVENT\r\nDTSTART;TZID=\"a\",\"b\":x\r\nEND:VEVENT\r\n", SCE_FORGE_CODEC_LINE_BAD_VALUE},
        {"BEGIN:VEVENT\r\nDTSTART;TZID=abcdefghi:x\r\nEND:VEVENT\r\n", SCE_FORGE_CODEC_LINE_TOO_LONG},
        {"BEGIN:VEVENT\r\nDTSTART;TZID=a\x02:x\r\nEND:VEVENT\r\n", SCE_FORGE_CODEC_LINE_BAD_VALUE},
    };

    size_t i;
    for (i = 0; i < sizeof cases / sizeof cases[0]; ++i) {
        sce_forge_cl_reader_t reader;
        sce_forge_cl_property_t line;
        bool has = false;
        bool more = false;
        char buf[8];
        size_t n = 0;
        (void)sce_forge_cl_reader_begin(&reader, (const uint8_t *)cases[i].text, strlen(cases[i].text), "VEVENT");
        (void)sce_forge_cl_reader_next(&reader, &line, &has);
        CHECK(sce_forge_cl_property_next_param(&line, &more) == SCE_FORGE_CODEC_OK && more, cases[i].text);
        CHECK(sce_forge_cl_property_read_param_string(&line, sizeof buf, buf, &n) == cases[i].want, cases[i].text);
    }
}

/* The property `N` holding `text`. */
static sce_forge_cl_property_t property_of(const char *text, char *storage, size_t storage_size) {
    sce_forge_cl_reader_t reader;
    sce_forge_cl_property_t line;
    bool has = false;
    const int n = snprintf(storage, storage_size, "BEGIN:VEVENT\r\nN:%s\r\nEND:VEVENT\r\n", text);
    (void)sce_forge_cl_reader_begin(&reader, (const uint8_t *)storage, (size_t)n, "VEVENT");
    (void)sce_forge_cl_reader_next(&reader, &line, &has);
    return line;
}

static sce_forge_codec_status_t uint_of(const char *text, uint64_t max, uint64_t *out) {
    char storage[128];
    sce_forge_cl_property_t p = property_of(text, storage, sizeof storage);
    return sce_forge_cl_property_read_uint(&p, max, out);
}

static sce_forge_codec_status_t int_of(const char *text, int64_t min, int64_t max, int64_t *out) {
    char storage[128];
    sce_forge_cl_property_t p = property_of(text, storage, sizeof storage);
    return sce_forge_cl_property_read_int(&p, min, max, out);
}

static sce_forge_codec_status_t bool_of(const char *text, bool *out) {
    char storage[128];
    sce_forge_cl_property_t p = property_of(text, storage, sizeof storage);
    return sce_forge_cl_property_read_bool(&p, out);
}

static void integers_and_booleans_hold_their_type_or_are_refused(void) {
    uint64_t u = 0;
    int64_t i = 0;
    bool b = false;
    CHECK(uint_of("255", 255, &u) == SCE_FORGE_CODEC_OK && u == 255U, "255 in 255");
    CHECK(uint_of("+007", 255, &u) == SCE_FORGE_CODEC_OK && u == 7U, "a plus sign and leading zeros");
    CHECK(uint_of("256", 255, &u) == SCE_FORGE_CODEC_LINE_BAD_VALUE, "256 in a uint8");
    CHECK(uint_of("-0", 255, &u) == SCE_FORGE_CODEC_LINE_BAD_VALUE, "a minus sign on an unsigned number");
    CHECK(uint_of("", 255, &u) == SCE_FORGE_CODEC_LINE_BAD_VALUE, "no digits");
    CHECK(uint_of("1x", 255, &u) == SCE_FORGE_CODEC_LINE_BAD_VALUE, "a letter after the digits");
    CHECK(uint_of(" 1", 255, &u) == SCE_FORGE_CODEC_LINE_BAD_VALUE, "a space before the digits");
    CHECK(uint_of("99999999999999999999999999999999999999999", UINT64_MAX, &u) == SCE_FORGE_CODEC_LINE_BAD_VALUE,
          "more digits than any type holds");
    CHECK(uint_of("18446744073709551615", UINT64_MAX, &u) == SCE_FORGE_CODEC_OK && u == UINT64_MAX,
          "the largest uint64");
    CHECK(uint_of("18446744073709551616", UINT64_MAX, &u) == SCE_FORGE_CODEC_LINE_BAD_VALUE,
          "one past the largest uint64");
    CHECK(int_of("-128", -128, 127, &i) == SCE_FORGE_CODEC_OK && i == -128, "-128 in an int8");
    CHECK(int_of("-129", -128, 127, &i) == SCE_FORGE_CODEC_LINE_BAD_VALUE, "-129 in an int8");
    CHECK(int_of("-0", -128, 127, &i) == SCE_FORGE_CODEC_OK && i == 0, "-0 in an int8");
    CHECK(int_of("-9223372036854775808", INT64_MIN, INT64_MAX, &i) == SCE_FORGE_CODEC_OK && i == INT64_MIN,
          "the smallest int64");
    CHECK(int_of("-9223372036854775809", INT64_MIN, INT64_MAX, &i) == SCE_FORGE_CODEC_LINE_BAD_VALUE,
          "one below the smallest int64");
    CHECK(int_of("9223372036854775808", INT64_MIN, INT64_MAX, &i) == SCE_FORGE_CODEC_LINE_BAD_VALUE,
          "one past the largest int64");
    CHECK(bool_of("true", &b) == SCE_FORGE_CODEC_OK && b, "true");
    CHECK(bool_of("False", &b) == SCE_FORGE_CODEC_OK && !b, "False");
    CHECK(bool_of("yes", &b) == SCE_FORGE_CODEC_LINE_BAD_VALUE, "yes");
    CHECK(bool_of("TRUEE", &b) == SCE_FORGE_CODEC_LINE_BAD_VALUE, "TRUEE");
}

static void a_value_is_read_once(void) {
    char storage[128];
    char buf[4];
    size_t n = 0;
    sce_forge_cl_property_t p = property_of("a", storage, sizeof storage);
    CHECK(sce_forge_cl_property_read_string(&p, 4, false, buf, &n) == SCE_FORGE_CODEC_OK, "the first read");
    CHECK(sce_forge_cl_property_read_string(&p, 4, false, buf, &n) == SCE_FORGE_CODEC_LINE_MALFORMED, "a second read");
}

/* What a writer writes, between `BEGIN:` and `END:`, into `buf`. */
typedef struct {
    uint8_t buf[1024];
    sce_forge_writer_t sink;
    sce_forge_cl_writer_t w;
} written_t;

static void written_init(written_t *out) {
    out->sink = sce_forge_writer_init_buf(out->buf, sizeof out->buf);
    out->w = sce_forge_cl_writer_init(&out->sink, "VEVENT");
    CHECK(sce_forge_cl_writer_begin(&out->w) == SCE_FORGE_CODEC_OK, "begin");
}

static size_t written_finish(written_t *out) {
    CHECK(sce_forge_cl_writer_finish(&out->w) == SCE_FORGE_CODEC_OK, "finish");
    return sce_forge_writer_position(&out->sink);
}

static void a_property_is_written_with_its_parameters_and_a_value_that_is_escaped(void) {
    static const char want[] =
        "BEGIN:VEVENT\r\nUID:a1\r\nDTSTART;TZID=Europe/Seoul;X-Q=\"a:b\":20260101T000000\r\n"
        "SUMMARY:a\\\\b\\;c\\,d\\ne\r\nN:18446744073709551615\r\nM:-9223372036854775808\r\nB:TRUE\r\nEND:VEVENT\r\n";
    written_t out;
    size_t n;
    written_init(&out);
    CHECK(sce_forge_cl_writer_property(&out.w, "UID") == SCE_FORGE_CODEC_OK, "name");
    CHECK(sce_forge_cl_writer_string(&out.w, "a1", 2, false, 8) == SCE_FORGE_CODEC_OK, "value");
    CHECK(sce_forge_cl_writer_property(&out.w, "DTSTART") == SCE_FORGE_CODEC_OK, "name");
    CHECK(sce_forge_cl_writer_param(&out.w, "TZID", "Europe/Seoul", 12, 64) == SCE_FORGE_CODEC_OK, "param");
    CHECK(sce_forge_cl_writer_param(&out.w, "X-Q", "a:b", 3, 8) == SCE_FORGE_CODEC_OK, "quoted");
    CHECK(sce_forge_cl_writer_string(&out.w, "20260101T000000", 15, false, 32) == SCE_FORGE_CODEC_OK, "value");
    CHECK(sce_forge_cl_writer_property(&out.w, "SUMMARY") == SCE_FORGE_CODEC_OK, "name");
    CHECK(sce_forge_cl_writer_string(&out.w, "a\\b;c,d\ne", 9, true, 32) == SCE_FORGE_CODEC_OK, "text");
    CHECK(sce_forge_cl_writer_property(&out.w, "N") == SCE_FORGE_CODEC_OK, "name");
    CHECK(sce_forge_cl_writer_uint(&out.w, UINT64_MAX) == SCE_FORGE_CODEC_OK, "uint");
    CHECK(sce_forge_cl_writer_property(&out.w, "M") == SCE_FORGE_CODEC_OK, "name");
    CHECK(sce_forge_cl_writer_int(&out.w, INT64_MIN) == SCE_FORGE_CODEC_OK, "int");
    CHECK(sce_forge_cl_writer_property(&out.w, "B") == SCE_FORGE_CODEC_OK, "name");
    CHECK(sce_forge_cl_writer_bool(&out.w, true) == SCE_FORGE_CODEC_OK, "bool");
    n = written_finish(&out);
    CHECK(n == sizeof want - 1U && memcmp(out.buf, want, n) == 0, "the written component");
}

static void a_line_is_cut_before_the_unit_that_would_pass_75_octets(void) {
    written_t out;
    char value[200];
    size_t n;
    size_t first;
    size_t second;
    size_t third;
    const uint8_t *p;
    memset(value, 'x', sizeof value);
    written_init(&out);
    CHECK(sce_forge_cl_writer_property(&out.w, "SUMMARY") == SCE_FORGE_CODEC_OK, "name");
    CHECK(sce_forge_cl_writer_string(&out.w, value, sizeof value, false, 256) == SCE_FORGE_CODEC_OK, "value");
    n = written_finish(&out);
    /* After "BEGIN:VEVENT\r\n" (14 octets): "SUMMARY:" takes 8 of the first 75,
     * leaving 67 of the 200; each continuation line carries its space and 74
     * more. */
    p = out.buf + 14;
    first = 0;
    while (p[first] != '\r') {
        ++first;
    }
    second = 0;
    p += first + 2U;
    while (p[second] != '\r') {
        ++second;
    }
    third = 0;
    p += second + 2U;
    while (p[third] != '\r') {
        ++third;
    }
    CHECK(n > 0, "something was written");
    CHECK(first == 75U && second == 75U && third == 1U + 200U - 67U - 74U, "fold widths");
}

static bool contains(const uint8_t *hay, size_t hay_len, const char *needle) {
    const size_t m = strlen(needle);
    size_t i;
    for (i = 0; i + m <= hay_len; ++i) {
        if (memcmp(hay + i, needle, m) == 0) {
            return true;
        }
    }
    return false;
}

static void a_cut_never_splits_a_character_or_an_escape(void) {
    written_t out;
    char s[80];
    char t[80];
    size_t n;
    memset(s, 'x', 71);
    memcpy(s + 71, "\xc3\xa9\xc3\xa9", 4);
    memset(t, 'x', 72);
    t[72] = ';';
    written_init(&out);
    CHECK(sce_forge_cl_writer_property(&out.w, "S") == SCE_FORGE_CODEC_OK, "name");
    CHECK(sce_forge_cl_writer_string(&out.w, s, 75, false, 256) == SCE_FORGE_CODEC_OK, "value");
    CHECK(sce_forge_cl_writer_property(&out.w, "T") == SCE_FORGE_CODEC_OK, "name");
    CHECK(sce_forge_cl_writer_string(&out.w, t, 73, true, 256) == SCE_FORGE_CODEC_OK, "value");
    n = written_finish(&out);
    {
        char want_s[96];
        char want_t[96];
        memcpy(want_s, "S:", 2);
        memset(want_s + 2, 'x', 71);
        memcpy(want_s + 73, "\xc3\xa9\r\n \xc3\xa9\r\n", 9);
        want_s[82] = '\0';
        CHECK(contains(out.buf, n, want_s), "a character stays whole");
        memcpy(want_t, "T:", 2);
        memset(want_t + 2, 'x', 72);
        memcpy(want_t + 74, "\r\n \\;\r\n", 7);
        want_t[81] = '\0';
        CHECK(contains(out.buf, n, want_t), "an escape stays whole");
    }
}

static void a_value_a_line_could_not_carry_is_refused_before_it_is_written(void) {
    written_t out;
    size_t before;
    written_init(&out);
    CHECK(sce_forge_cl_writer_property(&out.w, "P") == SCE_FORGE_CODEC_OK, "name");
    before = sce_forge_writer_position(&out.sink);
    CHECK(sce_forge_cl_writer_string(&out.w, "a\r\nATTENDEE:x", 13, false, 64) == SCE_FORGE_CODEC_LINE_BAD_VALUE,
          "a line break in a value");
    CHECK(sce_forge_cl_writer_string(&out.w, "a\nb", 3, false, 64) == SCE_FORGE_CODEC_LINE_BAD_VALUE,
          "a line feed outside a TEXT");
    CHECK(sce_forge_cl_writer_string(&out.w, "a\rb", 3, true, 64) == SCE_FORGE_CODEC_LINE_BAD_VALUE,
          "a carriage return in a TEXT");
    CHECK(sce_forge_cl_writer_string(&out.w, "abcd", 4, false, 3) == SCE_FORGE_CODEC_LINE_TOO_LONG, "past the size");
    CHECK(sce_forge_cl_writer_string(&out.w,
                                     "a\xff"
                                     "z",
                                     3, false, 64) == SCE_FORGE_CODEC_LINE_BAD_VALUE,
          "bytes that are not UTF-8");
    CHECK(sce_forge_cl_writer_param(&out.w, "Q", "a\"b", 3, 8) == SCE_FORGE_CODEC_LINE_BAD_VALUE,
          "a quote in a parameter");
    CHECK(sce_forge_cl_writer_param(&out.w, "Q", "a\nb", 3, 8) == SCE_FORGE_CODEC_LINE_BAD_VALUE,
          "a line feed in a parameter");
    CHECK(sce_forge_cl_writer_param(&out.w, "Q", "abcd", 4, 3) == SCE_FORGE_CODEC_LINE_TOO_LONG,
          "a parameter past its size");
    CHECK(sce_forge_writer_position(&out.sink) == before, "something of a refused value reached the sink");
}

static void a_full_sink_is_reported_as_the_sink_reports_it(void) {
    uint8_t buf[8];
    sce_forge_writer_t sink = sce_forge_writer_init_buf(buf, sizeof buf);
    sce_forge_cl_writer_t w = sce_forge_cl_writer_init(&sink, "VEVENT");
    CHECK(sce_forge_cl_writer_begin(&w) == SCE_FORGE_CODEC_BUFFER_OVERFLOW, "a sink of eight octets");
}

static void what_is_written_is_read_back(void) {
    static const char *const values[] = {"", "plain", "a;b,c\\d\ne", "caf\xc3\xa9 \xf0\x9f\x98\x80"};
    size_t k;
    for (k = 0; k < sizeof values / sizeof values[0]; ++k) {
        written_t out;
        sce_forge_cl_reader_t reader;
        sce_forge_cl_property_t line;
        bool has = false;
        char back[64];
        size_t back_len = 0;
        size_t n;
        written_init(&out);
        CHECK(sce_forge_cl_writer_property(&out.w, "SUMMARY") == SCE_FORGE_CODEC_OK, "name");
        CHECK(sce_forge_cl_writer_string(&out.w, values[k], strlen(values[k]), true, 64) == SCE_FORGE_CODEC_OK,
              "value");
        n = written_finish(&out);
        (void)sce_forge_cl_reader_begin(&reader, out.buf, n, "VEVENT");
        (void)sce_forge_cl_reader_next(&reader, &line, &has);
        CHECK(has &&
                  sce_forge_cl_property_read_string(&line, sizeof back, true, back, &back_len) == SCE_FORGE_CODEC_OK &&
                  back_len == strlen(values[k]) && memcmp(back, values[k], back_len) == 0,
              "the value read back");
        has = true;
        CHECK(sce_forge_cl_reader_next(&reader, &line, &has) == SCE_FORGE_CODEC_OK && !has, "the component ends");
        CHECK(sce_forge_cl_reader_consumed(&reader) == n, "all of it was consumed");
    }
}

int main(void) {
    a_component_is_read_property_by_property_and_stops_after_its_end();
    names_are_matched_without_regard_to_case_and_lf_ends_a_line();
    a_fold_is_removed_wherever_the_sender_cut();
    a_nested_component_and_an_undeclared_property_are_skipped_whole();
    an_input_that_ends_early_needs_more_bytes();
    a_line_the_grammar_does_not_admit_is_malformed();
    a_text_unescapes_and_a_bad_escape_is_refused();
    a_control_character_bad_utf8_and_a_long_value_are_refused();
    parameters_are_read_by_name_quoted_or_not_and_the_rest_are_skipped();
    a_parameter_of_two_values_or_past_its_bound_is_refused_where_it_is_read();
    integers_and_booleans_hold_their_type_or_are_refused();
    a_value_is_read_once();
    a_property_is_written_with_its_parameters_and_a_value_that_is_escaped();
    a_line_is_cut_before_the_unit_that_would_pass_75_octets();
    a_cut_never_splits_a_character_or_an_escape();
    a_value_a_line_could_not_carry_is_refused_before_it_is_written();
    a_full_sink_is_reported_as_the_sink_reports_it();
    what_is_written_is_read_back();
    if (failures != 0) {
        fprintf(stderr, "%d check(s) failed\n", failures);
        return 1;
    }
    return 0;
}
