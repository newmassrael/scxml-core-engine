// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A byte string rides in a payload as its byte-exact Latin-1 text (docs/adr/0005,
// decision 2): each byte is the character of its code point, so every one of the 256
// values is one character and back, and a character past U+00FF is no byte. The text is
// read the same whether a producer escapes what is not ASCII (a Python host's
// `json.dumps` does) or writes the characters as they are (nlohmann does), and the
// runtime's own writer is read back byte for byte.

#include <sce/event_payload.h>

#include <stdint.h>
#include <stdio.h>
#include <string.h>

enum { BUFFER = 8 };

static int failures = 0;

static void expect_bytes(const char *what, const char *wire, const uint8_t *want, size_t want_len) {
    sce_payload_fields_t fields = {wire};
    uint8_t out[BUFFER];
    size_t len = 0;
    const char *refusal = sce_payload_read_bytes(&fields, "frame", out, sizeof(out), &len);
    if (refusal != NULL) {
        fprintf(stderr, "FAIL: %s: refused as `%s`\n", what, refusal);
        failures++;
    } else if (len != want_len || memcmp(out, want, want_len) != 0) {
        fprintf(stderr, "FAIL: %s: read %zu bytes that are not the %zu wanted\n", what, len, want_len);
        failures++;
    }
}

static void expect_refusal(const char *what, const char *wire, const char *want) {
    sce_payload_fields_t fields = {wire};
    uint8_t out[BUFFER];
    size_t len = 0;
    const char *refusal = sce_payload_read_bytes(&fields, "frame", out, sizeof(out), &len);
    if (refusal == NULL) {
        fprintf(stderr, "FAIL: %s: read %zu bytes, wanted a refusal\n", what, len);
        failures++;
    } else if (strcmp(refusal, want) != 0) {
        fprintf(stderr, "FAIL: %s: refused as `%s`, wanted `%s`\n", what, refusal, want);
        failures++;
    }
}

int main(void) {
    static const uint8_t high_and_ascii[] = {0xFFu, 0x80u, 'a'};

    // A character written as itself and the escape that names it spell one byte.
    expect_bytes("escaped", "{\"frame\":\"\\u00ff\\u0080a\"}", high_and_ascii, sizeof(high_and_ascii));
    expect_bytes("written as itself",
                 "{\"frame\":\"\xC3\xBF\xC2\x80"
                 "a\"}",
                 high_and_ascii, sizeof(high_and_ascii));
    expect_bytes("written and escaped alike",
                 "{\"frame\":\"\xC3\xBF\\u0080"
                 "a\"}",
                 high_and_ascii, sizeof(high_and_ascii));
    expect_bytes("nothing at all", "{\"frame\":\"\"}", high_and_ascii, 0u);

    // A character past U+00FF is no byte, however it is spelled.
    expect_refusal("escaped past U+00FF", "{\"frame\":\"\\u0100\"}", SCE_PAYLOAD_REFUSE_NOT_ONE_BYTE);
    expect_refusal("written past U+00FF", "{\"frame\":\"\xC4\x80\"}", SCE_PAYLOAD_REFUSE_NOT_ONE_BYTE);
    expect_refusal("a euro sign", "{\"frame\":\"\xE2\x82\xAC\"}", SCE_PAYLOAD_REFUSE_NOT_ONE_BYTE);
    expect_refusal("an astral character", "{\"frame\":\"\xF0\x9F\x98\x80\"}", SCE_PAYLOAD_REFUSE_NOT_ONE_BYTE);
    expect_refusal("an astral character escaped", "{\"frame\":\"\\ud83d\\ude00\"}", SCE_PAYLOAD_REFUSE_NOT_ONE_BYTE);

    // What is not UTF-8 is no text.
    expect_refusal("a continuation alone", "{\"frame\":\"\x80\"}", SCE_PAYLOAD_REFUSE_NOT_TEXT);
    expect_refusal("a lead with no end", "{\"frame\":\"\xC3\"}", SCE_PAYLOAD_REFUSE_NOT_TEXT);
    expect_refusal("an overlong form", "{\"frame\":\"\xC0\x80\"}", SCE_PAYLOAD_REFUSE_NOT_TEXT);
    expect_refusal("a lead and no continuation", "{\"frame\":\"\xC3\x28\"}", SCE_PAYLOAD_REFUSE_NOT_TEXT);

    // The buffer is the schema's bound and holds no more.
    expect_refusal("past the buffer", "{\"frame\":\"123456789\"}", SCE_PAYLOAD_REFUSE_TOO_LONG);

    // The runtime's own writer is read back byte for byte.
    uint8_t everything[256];
    for (unsigned i = 0; i < 256u; ++i) {
        everything[i] = (uint8_t)i;
    }
    for (unsigned start = 0; start < 256u; start += BUFFER) {
        char quoted[BUFFER * 6 + 3];
        char wire[BUFFER * 6 + 32];
        if (!sce_payload_quote_bytes(everything + start, BUFFER, quoted, sizeof(quoted))) {
            fprintf(stderr, "FAIL: bytes from %u do not fit the quoter's buffer\n", start);
            failures++;
            continue;
        }
        (void)snprintf(wire, sizeof(wire), "{\"frame\":%s}", quoted);
        expect_bytes("the quoter's own text", wire, everything + start, BUFFER);
    }

    if (failures != 0) {
        fprintf(stderr, "%d failure(s)\n", failures);
        return 1;
    }
    return 0;
}
