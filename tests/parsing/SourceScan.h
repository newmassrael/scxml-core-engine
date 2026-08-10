// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

#pragma once

#include <fstream>
#include <gtest/gtest.h>
#include <regex>
#include <set>
#include <sstream>
#include <string>

// Source-text scanning shared by the parsing suites that DERIVE their
// claim from the tree instead of restating it.
//
// `Diagnostic_test` derives the declared wire codes from
// `sce/include/parsing/*Error.h`; `CrossProducerDiagnosticId_test`
// derives the declared `Diagnostic` leaf classes from the same
// headers. Both need the same three primitives, and a second copy of
// `stripComments` would be a second place for one rule to drift — the
// rule that keeps a code named in prose from counting as a code under
// test.

namespace SCE::TestSupport {

// Comments are stripped before any scan. A symbol named in prose is
// not a symbol under test, and — the direction that actually bit — the
// comment explaining a curated list would otherwise "exercise" every
// name it mentions, so the list could rot while its own explanation
// covered for it.
inline std::string stripComments(const std::string &src) {
    std::string out;
    out.reserve(src.size());
    for (std::size_t i = 0; i < src.size();) {
        if (src.compare(i, 2, "//") == 0) {
            const auto eol = src.find('\n', i);
            if (eol == std::string::npos) {
                break;
            }
            i = eol;  // keep the newline so line-oriented shapes survive
        } else if (src.compare(i, 2, "/*") == 0) {
            const auto end = src.find("*/", i + 2);
            i = (end == std::string::npos) ? src.size() : end + 2;
        } else {
            out.push_back(src[i]);
            ++i;
        }
    }
    return out;
}

// Capture group 1 of every match, de-duplicated.
inline std::set<std::string> matchAll(const std::string &text, const std::regex &pattern) {
    std::set<std::string> out;
    for (auto it = std::sregex_iterator{text.begin(), text.end(), pattern}; it != std::sregex_iterator{}; ++it) {
        out.insert((*it)[1].str());
    }
    return out;
}

inline std::string readFile(const std::string &path) {
    std::ifstream in{path};
    EXPECT_TRUE(in.is_open()) << "cannot open " << path;
    std::ostringstream buf;
    buf << in.rdbuf();
    return buf.str();
}

}  // namespace SCE::TestSupport
