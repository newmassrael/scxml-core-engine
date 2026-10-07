// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A byte string crosses a `<param>` and a `<donedata>` as its byte-exact Latin-1 text
// (docs/adr/0005, decision 2): each byte is the character of its code point, so every
// one of the 256 values is one character and back, a byte above 0x7F being the two
// bytes of UTF-8 its character is. A `sce-static` machine generated as C writes it
// through the header-only wire writer of the forge runtime, which holds it with its
// length because a C string is NUL-terminated and a byte string may hold a 0x00.
//
// What the writer wrote is read back with the payload reader the machine reads an event
// with, so the two halves are held to each other: the JSON text, the text a request
// carries (which cannot hold a 0x00 and is refused, not cut short), and what does not fit.

#include <sce/event_payload.h>
#include <sce/forge/wire.h>

#include <stdint.h>
#include <stdio.h>
#include <string.h>

enum { CHUNK = 16 };

static int failures = 0;

static void expect_text(const char *what, const char *got, const char *want) {
    if (strcmp(got, want) != 0) {
        (void)fprintf(stderr, "FAIL: %s: wrote `%s`, expected `%s`\n", what, got, want);
        failures++;
    }
}

// The bytes written as JSON and read back as the machine reads an event's data.
static void expect_round_trip(const char *what, const uint8_t *bytes, size_t len) {
    char json[CHUNK * 6 + 8];
    char wire[sizeof(json) + 16];
    uint8_t back[CHUNK];
    size_t back_len = 0;
    const sce_forge_bytes_view_t view = {bytes, len};
    if (!sce_forge_wire_json(sce_forge_wire_bytes(view), json, sizeof(json))) {
        (void)fprintf(stderr, "FAIL: %s: the JSON did not fit its buffer\n", what);
        failures++;
        return;
    }
    (void)snprintf(wire, sizeof(wire), "{\"frame\":%s}", json);
    sce_payload_fields_t fields = {wire};
    const char *refusal = sce_payload_read_bytes(&fields, "frame", back, sizeof(back), &back_len);
    if (refusal != NULL) {
        (void)fprintf(stderr, "FAIL: %s: read back as `%s`, from %s\n", what, refusal, wire);
        failures++;
    } else if (back_len != len || memcmp(back, bytes, len) != 0) {
        (void)fprintf(stderr, "FAIL: %s: %zu bytes read back, not the %zu written, from %s\n", what, back_len, len,
                      wire);
        failures++;
    }
}

int main(void) {
    char out[64];

    // Every value is one character and back, a 0x00 and a quote included.
    uint8_t everything[256];
    for (unsigned i = 0; i < 256u; ++i) {
        everything[i] = (uint8_t)i;
    }
    for (size_t at = 0; at < sizeof(everything); at += CHUNK) {
        expect_round_trip("all 256 values", everything + at, CHUNK);
    }
    expect_round_trip("no bytes", everything, 0u);

    // The JSON text itself: a quote, a backslash, a newline and a control byte are
    // escaped as any string's are, a byte above 0x7F is its character, and the rest
    // are as they are.
    static const uint8_t escaped[] = {'"', '\\', '\n', 0x01u, 0xE9u, 'a'};
    const sce_forge_bytes_view_t escaped_view = {escaped, sizeof(escaped)};
    if (!sce_forge_wire_json(sce_forge_wire_bytes(escaped_view), out, sizeof(out))) {
        (void)fprintf(stderr, "FAIL: the escapes did not fit their buffer\n");
        failures++;
    } else {
        expect_text("the escapes", out,
                    "\"\\\"\\\\\\n\\u0001\xC3\xA9"
                    "a\"");
    }
    const sce_forge_bytes_view_t none = {escaped, 0u};
    (void)sce_forge_wire_json(sce_forge_wire_bytes(none), out, sizeof(out));
    expect_text("no bytes", out, "\"\"");

    // The text a request carries: the Latin-1 text as UTF-8.
    static const uint8_t high[] = {'a', 0xFFu, 0x80u};
    const sce_forge_bytes_view_t high_view = {high, sizeof(high)};
    if (!sce_forge_wire_text(sce_forge_wire_bytes(high_view), out, sizeof(out))) {
        (void)fprintf(stderr, "FAIL: the text of the bytes above 0x7F did not fit\n");
        failures++;
    } else {
        expect_text("the text of the bytes above 0x7F", out, "a\xC3\xBF\xC2\x80");
    }

    // A C string holds no 0x00: a byte string that holds one is refused as text, never
    // cut short where the first would end it.
    static const uint8_t with_nul[] = {'a', 0x00u, 'b'};
    const sce_forge_bytes_view_t nul_view = {with_nul, sizeof(with_nul)};
    out[0] = 'x';
    if (sce_forge_wire_text(sce_forge_wire_bytes(nul_view), out, sizeof(out))) {
        (void)fprintf(stderr, "FAIL: a byte string holding a 0x00 was written as text\n");
        failures++;
    } else {
        expect_text("a byte string holding a 0x00, refused as text", out, "");
    }

    // What does not fit is refused whole, as every value is.
    static const uint8_t two[] = {0xFFu, 0xFFu};
    const sce_forge_bytes_view_t two_view = {two, sizeof(two)};
    char small[5];
    if (!sce_forge_wire_text(sce_forge_wire_bytes(two_view), small, sizeof(small))) {
        (void)fprintf(stderr, "FAIL: four bytes and a terminator did not fit five\n");
        failures++;
    }
    if (sce_forge_wire_text(sce_forge_wire_bytes(two_view), small, sizeof(small) - 1u)) {
        (void)fprintf(stderr, "FAIL: four bytes and a terminator fitted four\n");
        failures++;
    } else {
        expect_text("what does not fit", small, "");
    }

    if (failures != 0) {
        (void)fprintf(stderr, "%d failure(s)\n", failures);
        return 1;
    }
    return 0;
}
