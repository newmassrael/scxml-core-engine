// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

#include "parsing/DiagnosticBatchFormatter.h"

#include <ostream>

namespace SCE::parsing {

void emit_json_diagnostics(
    const std::vector<std::unique_ptr<Diagnostic>> &diagnostics,
    std::ostream &os) {
    // NDJSON line shape: each call to `to_json().dump()` produces
    // one record body with no trailing newline, then we append
    // '\n' to delimit. No array wrapper, no separator commas —
    // mirroring `serde_json::to_string` + `writeln!` on the Rust
    // side (sce-build/src/forge/diagnostic.rs::emit_json).
    //
    // Skip null entries defensively. `recordDiagnostic()` filters
    // null on the producer side, but a direct caller assembling
    // the vector by hand could leave a hole; emitting an empty
    // line would corrupt the line-based reader.
    for (const auto &diag : diagnostics) {
        if (!diag) {
            continue;
        }
        os << diag->to_json().dump() << '\n';
    }
}

}  // namespace SCE::parsing
