// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// sce/forge/text_template.h — the fence of a text template's untrusted hole
// (docs/adr/0087, section 3.1). The cases every runtime's removal is held to
// (backends/rust/forge-runtime/src/text_template.rs): a value with no marker is
// kept whole; each marker is removed wherever it stands; a marker the removal
// rebuilds is removed too; and multi-byte text beside a marker is kept as it was.

#include "sce/forge/text_template.h"

#include <cstdio>
#include <string>

namespace {

int failures = 0;

void check(const std::string &got, const std::string &want, const char *what) {
    if (got != want) {
        ++failures;
        std::fprintf(stderr, "FAIL %s: got \"%s\", want \"%s\"\n", what, got.c_str(), want.c_str());
    }
}

}  // namespace

int main() {
    const struct {
        const char *value;
        const char *kept;
    } cases[] = {
        {"plain words", "plain words"},
        {"a<<<TEXTb", "ab"},
        {"aTEXT>>>b<<<TEXTc", "abc"},
        {"<<<TE<<<TEXTXT", ""},
        {"TEXT>>TEXT>>>>", ""},
        {"\xc3\xa9<<<TEXT\xc3\xbc", "\xc3\xa9\xc3\xbc"},
        {"", ""},
    };

    for (const auto &c : cases) {
        check(SCE::Forge::removeMarkers(c.value, "<<<TEXT", "TEXT>>>"), c.kept, c.value);
    }
    // Where both markers start at one byte, the longer is removed.
    check(SCE::Forge::removeMarkers("abcd", "ab", "abc"), "d", "longer first");
    // An empty marker removes nothing, and the other is still removed.
    check(SCE::Forge::removeMarkers("x>>y", "", ">>"), "xy", "empty open");
    check(SCE::Forge::removeMarkers("abc", "", ""), "abc", "both empty");
    if (failures != 0) {
        std::fprintf(stderr, "%d failure(s)\n", failures);
        return 1;
    }
    return 0;
}
