// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

#pragma once

#include "parsing/Diagnostic.h"

#include <iosfwd>
#include <memory>
#include <vector>

// Batch NDJSON formatter for typed `SCE::parsing::Diagnostic`
// records. Each diagnostic emits one JSON record per line via
// `Diagnostic::to_json().dump()` followed by `'\n'` — same shape
// the Rust producer's `--error-format=json` flag emits, so a
// downstream consumer (telemetry, IDE, repair consumer) can read
// either side's stream with the same line-based parser.
//
// `SCXMLParser::getDiagnostics()` is the typical input source; the
// formatter does not depend on `SCXMLParser` so any consumer with
// its own diagnostic vector can call this directly. §wire-W2
// deliverable item #2.

namespace SCE::parsing {

void emit_json_diagnostics(const std::vector<std::unique_ptr<Diagnostic>> &diagnostics, std::ostream &os);

}  // namespace SCE::parsing
