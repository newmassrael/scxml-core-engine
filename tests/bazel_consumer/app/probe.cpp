// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Bazel consumer probe: a compiled sce_base symbol and a Forge C++ runtime
// header, both reached through `@sce//:sce_base` alone. The Forge header is the
// one the generated C++ includes (sce/forge/span.h), so a channel that stops
// carrying the runtime fails here at compile time; a channel that stops
// building sce_base fails at link time.

#include "common/UniqueIdGenerator.h"

#include <sce/forge/span.h>

#include <string>
#include <vector>

int main() {
    const std::vector<int> values{1, 2, 3};
    const SCE::Forge::Span<const int> view(values);
    const std::string id = SCE::UniqueIdGenerator::generateSessionId();
    return (view.size() == 3 && view[2] == 3 && !id.empty()) ? 0 : 1;
}
