// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Why a recorded input trace, or a value of one, is not read
// (docs/SCE_TTCN3_LANE.md §10.2): the caller's input, refused by name before
// anything of the run happens. The trace's reader (test/Trace.h) and the
// readers of the canonical form (test/FromCanonical.h) throw it, and the unit
// of a test configuration's `input` throws it for an input its system ports do
// not take. The Rust sibling is `sce_rust_runtime::test::TraceRefusal`, whose
// reasons these are, word for word.

#pragma once

#include <algorithm>
#include <stdexcept>
#include <string>
#include <vector>

namespace SCE::Test {

/// Why a trace, or a value of one, is not read.
class TraceRefusal : public std::runtime_error {
public:
    /// Refuse, saying why.
    explicit TraceRefusal(const std::string &reason) : std::runtime_error(reason) {}

    /// Why.
    std::string reason() const {
        return what();
    }

    /// An input of the record type @p typeName on @p systemPort, which the
    /// configuration whose system ports are @p systemPorts does not take: the
    /// port is none of them, or its type's `in` list does not name the type
    /// (docs/SCE_TTCN3_LANE.md §10.2).
    static TraceRefusal notAnInput(const std::string &systemPort, const std::string &typeName,
                                   const std::vector<std::string> &systemPorts) {
        if (std::find(systemPorts.begin(), systemPorts.end(), systemPort) != systemPorts.end()) {
            return TraceRefusal("'" + typeName + "' is not on the `in` list of the type of the system port '" +
                                systemPort + "'");
        }
        return TraceRefusal("'" + systemPort + "' is not a system port of the test case's configuration");
    }
};

}  // namespace SCE::Test
