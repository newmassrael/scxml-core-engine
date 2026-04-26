// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

#include "parsing/Diagnostic.h"

#include <nlohmann/json.hpp>

#include <string>

namespace SCE::parsing {

std::string Diagnostic::to_canonical_json_string() const {
    // `to_json()` returns `nlohmann::ordered_json` — keys are emitted
    // in producer insertion order, which is convenient for human
    // reading but unstable as a byte-diff target. Re-parse through
    // the default `nlohmann::json` (std::map-backed) to alphabetise
    // keys, then dump with `dump(-1, ' ', false)`:
    //   indent = -1 → no whitespace between tokens
    //   indent_char = ' ' → unused (indent = -1)
    //   ensure_ascii = false → preserve UTF-8 bytes verbatim
    // The result is the canonical NDJSON record body the W1 byte-
    // diff parity test pins against Rust's --error-format=json
    // output. RFC §W2 deliverable item #3.
    const auto record = to_json();
    const auto canonical =
        nlohmann::json::parse(record.dump());
    return canonical.dump(-1, ' ', false);
}

}  // namespace SCE::parsing
