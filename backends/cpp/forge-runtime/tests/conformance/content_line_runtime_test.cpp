// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// sce/forge/content_line.h — the content lines a `sce:encoding="content-line"`
// codec reads and writes (SCE_FORGE.md §4.6.4). The same properties the Rust
// runtime's own tests pin (backends/rust/forge-runtime/src/content_line.rs):
// unfolding wherever the sender cut, names read without regard to case, a
// nested component and an undeclared property skipped whole, truncation told
// from malformation, TEXT escapes, a value held to its type, and a writer that
// folds before the unit that would pass 75 octets and refuses a value a line
// could not carry. What a generated codec does with these is the conformance
// harness's (codec_content_line_event), which this file does not repeat.

#include "sce/forge/content_line.h"

#include <cstdint>
#include <cstdio>
#include <string>
#include <vector>

namespace {

int failures = 0;

#define CHECK(cond, what)                                                                                              \
    do {                                                                                                               \
        if (!(cond)) {                                                                                                 \
            ++failures;                                                                                                \
            std::fprintf(stderr, "FAIL %s:%d %s\n", __FILE__, __LINE__, what);                                         \
        }                                                                                                              \
    } while (0)

namespace cl = ::SCE::Forge::ContentLine;
using ::SCE::Forge::CodecError;
using ::SCE::Forge::VectorSink;
using Bytes = std::vector<std::uint8_t>;

Bytes bytes_of(const std::string &s) {
    return Bytes(s.begin(), s.end());
}

/// The properties of a VEVENT read by name, as (name, value) pairs; or the rule
/// that refused it.
struct Outcome {
    std::vector<std::pair<std::string, std::string>> seen;
    std::optional<CodecError> error;
};

Outcome read_all(const std::string &text) {
    const Bytes in = bytes_of(text);
    Outcome out;
    auto reader = cl::Reader::begin(in.data(), in.size(), "VEVENT");
    if (!reader) {
        out.error = CodecError::NeedMoreBytes;
        return out;
    }
    for (;;) {
        auto line = reader->next();
        if (!line) {
            if (reader->failed()) {
                out.error = reader->error();
            }
            return out;
        }
        const char *name = line->is("UID") ? "UID" : line->is("SUMMARY") ? "SUMMARY" : nullptr;
        if (name == nullptr) {
            continue;
        }
        auto value = line->read_string(32, std::string(name) == "SUMMARY");
        if (!value) {
            out.error = line->error();
            return out;
        }
        out.seen.emplace_back(name, *value);
    }
}

/// `SUMMARY:<value>` inside a VEVENT.
Outcome one(const std::string &value) {
    return read_all("BEGIN:VEVENT\r\nSUMMARY:" + value + "\r\nEND:VEVENT\r\n");
}

bool reads(const Outcome &o, const std::string &name, const std::string &value) {
    return !o.error && o.seen.size() == 1 && o.seen[0].first == name && o.seen[0].second == value;
}

void a_component_is_read_property_by_property_and_stops_after_its_end() {
    const std::string text =
        "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VEVENT\r\nUID:a1\r\nSUMMARY:hi\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
    const Bytes in = bytes_of(text);
    auto reader = cl::Reader::begin(in.data(), in.size(), "VEVENT");
    CHECK(reader.has_value(), "the component was not found");
    int seen = 0;
    while (reader) {
        auto line = reader->next();
        if (!line) {
            break;
        }
        CHECK(line->is("UID") || line->is("SUMMARY"), "a property nobody declared surfaced");
        ++seen;
    }
    CHECK(seen == 2, "two properties");
    CHECK(reader && !reader->failed(), "the end was a refusal");
    CHECK(reader && std::string(text.begin() + static_cast<std::ptrdiff_t>(reader->consumed()), text.end()) ==
                        "END:VCALENDAR\r\n",
          "the walk did not stop after END:VEVENT");
}

void names_are_matched_without_regard_to_case_and_lf_ends_a_line() {
    const auto o = read_all("begin:vevent\nuid:x\nend:Vevent\n");
    CHECK(reads(o, "UID", "x"), "a lower-case LF component");
}

void a_fold_is_removed_wherever_the_sender_cut() {
    CHECK(reads(read_all("BEGIN:VEVENT\r\nUI\r\n D:ab\r\n\tcd\r\n e\r\nEND:VEVENT\r\n"), "UID", "abcde"),
          "a fold inside a name and inside a value");
    CHECK(reads(one(std::string("\xc3\r\n \xa9", 5)), "SUMMARY", "\xc3\xa9"), "a cut inside a UTF-8 character");
}

void a_nested_component_and_an_undeclared_property_are_skipped_whole() {
    const auto o = read_all("BEGIN:VEVENT\r\nX-NOISE;A=\"b:c\":d\r\nBEGIN:VALARM\r\nUID:inner\r\nBEGIN:X\r\nEND:X\r\n"
                            "END:VALARM\r\nUID:outer\r\nEND:VEVENT\r\n");
    CHECK(reads(o, "UID", "outer"), "the nested component's own UID surfaced");
}

void an_input_that_ends_early_needs_more_bytes() {
    const char *inputs[] = {"",
                            "BEGIN:VCALENDAR\r\n",
                            "BEGIN:VEVENT",
                            "BEGIN:VEVENT\r\nUID:a",
                            "BEGIN:VEVENT\r\nUID:a\r\n",
                            "BEGIN:VEVENT\r\nUID:a\r\nEND:VEVEN",
                            "BEGIN:VEVENT\r\nBEGIN:VALARM\r\nEND:VALARM\r\n"};
    for (const char *input : inputs) {
        const auto o = read_all(input);
        CHECK(o.error == CodecError::NeedMoreBytes, "a truncated input was not NeedMoreBytes");
    }
    CHECK(!read_all("BEGIN:VEVENT\r\nUID:a\r\nEND:VEVENT").error, "the END line needs no line break after it");
}

void a_line_the_grammar_does_not_admit_is_malformed() {
    const char *inputs[] = {"BEGIN:VEVENT\r\n\r\nEND:VEVENT\r\n",
                            "BEGIN:VEVENT\r\nUID\r\nEND:VEVENT\r\n",
                            "BEGIN:VEVENT\r\n=x:y\r\nEND:VEVENT\r\n",
                            "BEGIN:VEVENT\r\nUID:a\r\nEND:VTODO\r\n",
                            "BEGIN:VEVENT\r\nUID;TZID:a\r\nEND:VEVENT\r\n",
                            "BEGIN:VEVENT\r\nUID;TZID=\"a:b\r\nEND:VEVENT\r\n",
                            "BEGIN:VEVENT\r\nUID;TZID=\"a\"b:c\r\nEND:VEVENT\r\n",
                            "BEGIN:VEVENT\r\nUID;TZID=a\"b:c\r\nEND:VEVENT\r\n"};
    for (const char *input : inputs) {
        const auto o = read_all(input);
        CHECK(o.error == CodecError::LineMalformed, input);
    }
}

void a_text_unescapes_and_a_bad_escape_is_refused() {
    CHECK(reads(one("a\\\\b\\;c\\,d\\ne\\Nf;g,h"), "SUMMARY", "a\\b;c,d\ne\nf;g,h"), "every TEXT escape");
    CHECK(one("a\\xb").error == CodecError::LineBadEscape, "an unknown escape");
    CHECK(one("a\\").error == CodecError::LineBadEscape, "a backslash that ends the value");
    const Bytes in = bytes_of("BEGIN:VEVENT\r\nUID:a\\nb\r\nEND:VEVENT\r\n");
    auto reader = cl::Reader::begin(in.data(), in.size(), "VEVENT");
    auto line = reader->next();
    const auto value = line->read_string(8, false);
    CHECK(value && *value == "a\\nb", "without sce:value=\"text\" a backslash is a byte like another");
}

void a_control_character_bad_utf8_and_a_long_value_are_refused() {
    CHECK(one("a\x01"
              "b")
                  .error == CodecError::LineBadValue,
          "a control character");
    CHECK(one("a\rb").error == CodecError::LineBadValue, "a carriage return");
    CHECK(one("a\x7f"
              "b")
                  .error == CodecError::LineBadValue,
          "a delete character");
    CHECK(one("a\xff"
              "b")
                  .error == CodecError::LineBadValue,
          "bytes that are not UTF-8");
    CHECK(!one("a\tb").error, "a tab is a character like another");
    CHECK(one(std::string(33, 'x')).error == CodecError::LineTooLong, "33 octets past 32");
    CHECK(!one(std::string(32, 'x')).error, "32 octets in 32");
    std::string escaped;
    for (int i = 0; i < 32; ++i) {
        escaped += "\\;";
    }
    CHECK(!one(escaped).error, "the size is that of the unescaped text");
}

void parameters_are_read_by_name_quoted_or_not_and_the_rest_are_skipped() {
    const Bytes in =
        bytes_of("BEGIN:VEVENT\r\nDTSTART;X-A=1;TZID=\"Europe/Seoul:KST\";X-B=\"p,q\",r:20260101T000000\r\n"
                 "END:VEVENT\r\n");
    auto reader = cl::Reader::begin(in.data(), in.size(), "VEVENT");
    auto line = reader->next();
    CHECK(line && line->is("dtstart"), "the property");
    std::optional<std::string> tzid;
    for (;;) {
        const auto more = line->next_param();
        CHECK(more.has_value(), "a parameter was refused");
        if (!more || !*more) {
            break;
        }
        if (line->param_is("tzid")) {
            tzid = line->read_param_string(32);
        }
    }
    CHECK(tzid && *tzid == "Europe/Seoul:KST", "the quoted parameter");
    const auto value = line->read_string(32, false);
    CHECK(value && *value == "20260101T000000", "the value after the parameters");
    CHECK(!reader->next() && !reader->failed(), "the component ends after it");
}

void a_parameter_of_two_values_or_past_its_bound_is_refused_where_it_is_read() {
    struct Case {
        const char *param;
        CodecError want;
    };

    const Case cases[] = {{"TZID=a,b", CodecError::LineBadValue},
                          {"TZID=\"a\",\"b\"", CodecError::LineBadValue},
                          {"TZID=abcdefghi", CodecError::LineTooLong},
                          {"TZID=a\x02", CodecError::LineBadValue}};
    for (const auto &c : cases) {
        const Bytes in = bytes_of(std::string("BEGIN:VEVENT\r\nDTSTART;") + c.param + ":x\r\nEND:VEVENT\r\n");
        auto reader = cl::Reader::begin(in.data(), in.size(), "VEVENT");
        auto line = reader->next();
        const auto more = line->next_param();
        CHECK(more && *more, "the parameter was not found");
        CHECK(!line->read_param_string(8) && line->error() == c.want, c.param);
    }
}

/// Read the value `text` of a property `N`.
cl::Property property_of(const Bytes &in) {
    auto reader = cl::Reader::begin(in.data(), in.size(), "VEVENT");
    return *reader->next();
}

void integers_and_booleans_hold_their_type_or_are_refused() {
    auto uint_of = [](const char *text, std::uint64_t max) {
        const Bytes in = bytes_of(std::string("BEGIN:VEVENT\r\nN:") + text + "\r\nEND:VEVENT\r\n");
        auto p = property_of(in);
        return p.read_uint(max);
    };
    auto int_of = [](const char *text, std::int64_t min, std::int64_t max) {
        const Bytes in = bytes_of(std::string("BEGIN:VEVENT\r\nN:") + text + "\r\nEND:VEVENT\r\n");
        auto p = property_of(in);
        return p.read_int(min, max);
    };
    auto bool_of = [](const char *text) {
        const Bytes in = bytes_of(std::string("BEGIN:VEVENT\r\nN:") + text + "\r\nEND:VEVENT\r\n");
        auto p = property_of(in);
        return p.read_bool();
    };
    CHECK(uint_of("255", 255) == 255U, "255 in 255");
    CHECK(uint_of("+007", 255) == 7U, "a plus sign and leading zeros");
    CHECK(!uint_of("256", 255), "256 in a uint8");
    CHECK(!uint_of("-0", 255), "a minus sign on an unsigned number");
    CHECK(!uint_of("", 255), "no digits");
    CHECK(!uint_of("1x", 255), "a letter after the digits");
    CHECK(!uint_of(" 1", 255), "a space before the digits");
    CHECK(!uint_of("99999999999999999999999999999999999999999", UINT64_MAX), "more digits than any type holds");
    CHECK(uint_of("18446744073709551615", UINT64_MAX) == UINT64_MAX, "the largest uint64");
    CHECK(!uint_of("18446744073709551616", UINT64_MAX), "one past the largest uint64");
    CHECK(int_of("-128", -128, 127) == -128, "-128 in an int8");
    CHECK(!int_of("-129", -128, 127), "-129 in an int8");
    CHECK(int_of("-9223372036854775808", INT64_MIN, INT64_MAX) == INT64_MIN, "the smallest int64");
    CHECK(!int_of("-9223372036854775809", INT64_MIN, INT64_MAX), "one below the smallest int64");
    CHECK(!int_of("9223372036854775808", INT64_MIN, INT64_MAX), "one past the largest int64");
    CHECK(bool_of("true") == true, "true");
    CHECK(bool_of("False") == false, "False");
    CHECK(!bool_of("yes"), "yes");
    CHECK(!bool_of("TRUEE"), "TRUEE");
}

void a_value_is_read_once() {
    const Bytes in = bytes_of("BEGIN:VEVENT\r\nUID:a\r\nEND:VEVENT\r\n");
    auto p = property_of(in);
    CHECK(p.read_string(4, false).has_value(), "the first read");
    CHECK(!p.read_string(4, false) && p.error() == CodecError::LineMalformed, "a second read");
}

/// What a writer writes between `BEGIN:` and `END:`.
template <typename F> std::string written(F write) {
    Bytes out;
    VectorSink sink(out);
    cl::Writer w(sink, "VEVENT");
    CHECK(!w.begin().has_value(), "begin");
    write(w);
    CHECK(!w.finish().has_value(), "finish");
    return std::string(out.begin(), out.end());
}

void a_property_is_written_with_its_parameters_and_a_value_that_is_escaped() {
    const std::string text = written([](cl::Writer &w) {
        CHECK(!w.property("UID"), "name");
        CHECK(!w.string("a1", false, 8), "value");
        CHECK(!w.property("DTSTART"), "name");
        CHECK(!w.param("TZID", "Europe/Seoul", 64), "param");
        CHECK(!w.param("X-Q", "a:b", 8), "quoted");
        CHECK(!w.string("20260101T000000", false, 32), "value");
        CHECK(!w.property("SUMMARY"), "name");
        CHECK(!w.string("a\\b;c,d\ne", true, 32), "text");
        CHECK(!w.property("N"), "name");
        CHECK(!w.uint(UINT64_MAX), "uint");
        CHECK(!w.property("M"), "name");
        CHECK(!w.integer(INT64_MIN), "int");
        CHECK(!w.property("B"), "name");
        CHECK(!w.boolean(true), "bool");
    });
    CHECK(text == "BEGIN:VEVENT\r\nUID:a1\r\nDTSTART;TZID=Europe/Seoul;X-Q=\"a:b\":20260101T000000\r\n"
                  "SUMMARY:a\\\\b\\;c\\,d\\ne\r\nN:18446744073709551615\r\nM:-9223372036854775808\r\nB:TRUE\r\n"
                  "END:VEVENT\r\n",
          "the written component");
}

std::vector<std::string> lines_of(const std::string &text) {
    std::vector<std::string> lines;
    std::size_t at = 0;
    for (;;) {
        const auto end = text.find("\r\n", at);
        if (end == std::string::npos) {
            return lines;
        }
        lines.push_back(text.substr(at, end - at));
        at = end + 2;
    }
}

void a_line_is_cut_before_the_unit_that_would_pass_75_octets() {
    const std::string text = written([](cl::Writer &w) {
        CHECK(!w.property("SUMMARY"), "name");
        CHECK(!w.string(std::string(200, 'x'), false, 256), "value");
    });
    const auto lines = lines_of(text);
    CHECK(lines.size() == 5, "five lines");
    // "SUMMARY:" takes 8 of the first 75 octets, leaving 67 of the 200; each
    // continuation line carries its space and 74 more.
    CHECK(lines[1].size() == 75, "the first line");
    CHECK(lines[2].size() == 75 && lines[2][0] == ' ', "the second line");
    CHECK(lines[3].size() == 1 + 200 - 67 - 74 && lines[3][0] == ' ', "the third line");
}

void a_cut_never_splits_a_character_or_an_escape() {
    const std::string x71(71, 'x');
    const std::string text = written([&](cl::Writer &w) {
        CHECK(!w.property("S"), "name");
        CHECK(!w.string(x71 + "\xc3\xa9\xc3\xa9", false, 256), "value");
        CHECK(!w.property("T"), "name");
        CHECK(!w.string(std::string(72, 'x') + ";", true, 256), "value");
    });
    CHECK(text.find("S:" + x71 + "\xc3\xa9\r\n \xc3\xa9\r\n") != std::string::npos, "a character stays whole");
    CHECK(text.find("T:" + std::string(72, 'x') + "\r\n \\;\r\n") != std::string::npos, "an escape stays whole");
    const Bytes in = bytes_of(text);
    auto reader = cl::Reader::begin(in.data(), in.size(), "VEVENT");
    auto s = reader->next();
    const auto sv = s->read_string(80, false);
    CHECK(sv && *sv == x71 + "\xc3\xa9\xc3\xa9", "S reads back");
    auto t = reader->next();
    const auto tv = t->read_string(80, true);
    CHECK(tv && *tv == std::string(72, 'x') + ";", "T reads back");
}

void a_value_a_line_could_not_carry_is_refused_before_it_is_written() {
    Bytes out;
    VectorSink sink(out);
    cl::Writer w(sink, "VEVENT");
    CHECK(!w.begin(), "begin");
    CHECK(!w.property("P"), "name");
    const auto before = out.size();
    CHECK(w.string("a\r\nATTENDEE:x", false, 64) == CodecError::LineBadValue, "a line break in a value");
    CHECK(w.string("a\nb", false, 64) == CodecError::LineBadValue, "a line feed outside a TEXT");
    CHECK(w.string("a\rb", true, 64) == CodecError::LineBadValue, "a carriage return in a TEXT");
    CHECK(w.string("abcd", false, 3) == CodecError::LineTooLong, "past the size");
    CHECK(w.string("a\xffz", false, 64) == CodecError::LineBadValue, "bytes that are not UTF-8");
    CHECK(w.param("Q", "a\"b", 8) == CodecError::LineBadValue, "a quote in a parameter");
    CHECK(w.param("Q", "a\nb", 8) == CodecError::LineBadValue, "a line feed in a parameter");
    CHECK(w.param("Q", "abcd", 3) == CodecError::LineTooLong, "a parameter past its size");
    CHECK(out.size() == before, "something of a refused value reached the sink");
}

void a_full_sink_is_reported_as_the_sink_reports_it() {
    std::uint8_t buf[8];
    ::SCE::Forge::SpanSink sink(buf, sizeof buf);
    cl::Writer w(sink, "VEVENT");
    CHECK(w.begin() == CodecError::BufferOverflow, "a sink of eight octets");
}

void what_is_written_is_read_back() {
    const std::string values[] = {"", "plain", "a;b,c\\d\ne", "caf\xc3\xa9 \xf0\x9f\x98\x80", std::string(150, 'y')};
    for (const auto &value : values) {
        const std::string text = written([&](cl::Writer &w) {
            CHECK(!w.property("SUMMARY"), "name");
            CHECK(!w.string(value, true, 256), "value");
        });
        const Bytes in = bytes_of(text);
        auto reader = cl::Reader::begin(in.data(), in.size(), "VEVENT");
        auto line = reader->next();
        const auto back = line->read_string(256, true);
        CHECK(back && *back == value, "the value read back");
        CHECK(!reader->next() && !reader->failed(), "the component ends");
        CHECK(reader->consumed() == text.size(), "all of it was consumed");
    }
}

}  // namespace

int main() {
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
        std::fprintf(stderr, "%d check(s) failed\n", failures);
        return 1;
    }
    return 0;
}
