/* SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael */

/*
 * sce/forge/text_template.h — the writer a text-template unit renders into
 * (docs/adr/0087, section 3.1). The removal of the fence markers is held to the
 * cases every runtime's removal is held to
 * (backends/rust/forge-runtime/src/text_template.rs); the writer to the digits
 * of each integer width's ends, to a fenced value between its markers, and to a
 * rendering that does not fit, which leaves "" and is answered as such.
 */

#include "sce/forge/text_template.h"

#include <stdint.h>
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

static void removed(const char *value, const char *kept) {
    char held[64];
    size_t len = strlen(value);
    memcpy(held, value, len);
    len = sce_forge_text_remove_markers(held, len, "<<<TEXT", "TEXT>>>");
    held[len] = '\0';
    CHECK(strcmp(held, kept) == 0, value);
}

int main(void) {
    removed("plain words", "plain words");
    removed("a<<<TEXTb", "ab");
    removed("aTEXT>>>b<<<TEXTc", "abc");
    removed("<<<TE<<<TEXTXT", "");
    removed("TEXT>>TEXT>>>>", "");
    removed("\xc3\xa9<<<TEXT\xc3\xbc", "\xc3\xa9\xc3\xbc");
    removed("", "");

    /* Where both markers start at one byte, the longer is removed. */
    {
        char held[] = "abcd";
        size_t len = sce_forge_text_remove_markers(held, 4u, "ab", "abc");
        CHECK(len == 1u && held[0] == 'd', "the longer marker first");
    }
    /* An empty marker removes nothing, and the other is still removed. */
    {
        char held[] = "x>>y";
        size_t len = sce_forge_text_remove_markers(held, 4u, "", ">>");
        CHECK(len == 2u && memcmp(held, "xy", 2u) == 0, "an empty open marker");
        CHECK(sce_forge_text_remove_markers(held, 2u, "", "") == 2u, "two empty markers");
    }

    /* Each integer width's ends, a fenced value, and the terminator. */
    {
        char buf[96];
        sce_forge_text_t text = sce_forge_text_begin(buf, sizeof buf);
        sce_forge_text_put_uint(&text, UINT64_MAX);
        sce_forge_text_put(&text, " ");
        sce_forge_text_put_int(&text, INT64_MIN);
        sce_forge_text_put(&text, " ");
        sce_forge_text_put_int(&text, 0);
        sce_forge_text_put_fenced(&text, "a<<<TE<<<TEXTXTb", "<<<TEXT", "TEXT>>>");
        CHECK(sce_forge_text_end(&text), "a rendering that fits");
        CHECK(strcmp(buf, "18446744073709551615 -9223372036854775808 0<<<TEXTabTEXT>>>") == 0, buf);
    }

    /* A rendering past its buffer leaves "" and says so, and the machine's read
     * of it records a capacity failure. */
    {
        char buf[8];
        sce_forge_text_t text = sce_forge_text_begin(buf, sizeof buf);
        sce_forge_text_put(&text, "1234567");
        CHECK(!text.overflow, "seven bytes and the terminator fit eight");
        sce_forge_text_put(&text, "8");
        CHECK(!sce_forge_text_end(&text), "a rendering past its buffer");
        CHECK(buf[0] == '\0', "a rendering past its buffer leaves the empty text");

        sce_forge_algorithm_failure_t failure = {0};
        CHECK(strcmp(sce_forge_text_rendered(&failure, NULL), "") == 0, "no rendering reads as empty");
        CHECK(failure.failed, "no rendering is a failure");
        sce_forge_algorithm_failure_t none = {0};
        CHECK(strcmp(sce_forge_text_rendered(&none, "kept"), "kept") == 0, "a rendering is read as it is");
        CHECK(!none.failed, "a rendering is no failure");
    }

    if (failures != 0) {
        fprintf(stderr, "%d failure(s)\n", failures);
        return 1;
    }
    return 0;
}
