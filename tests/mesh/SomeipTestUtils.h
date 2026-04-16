// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// vsomeip-specific test utilities layered on top of MeshTestUtils.h.

#pragma once

#include <filesystem>
#include <system_error>

namespace SCE::Test::Mesh {

// Wipe stale vsomeip local endpoints left by crashed/killed prior runs.
// vsomeip's routing manager refuses to bind when a previous socket owner
// is gone but the path survives. Using the filesystem API (not
// system("rm")) keeps the test hermetic with no shell dependency. Errors
// from missing/already-unlinked files are ignored.
inline void wipe_stale_vsomeip_sockets() {
    std::error_code ec;
    for (const auto& entry : std::filesystem::directory_iterator("/tmp", ec)) {
        if (ec) return;
        const auto name = entry.path().filename().string();
        if (name.rfind("vsomeip-", 0) == 0) {
            std::filesystem::remove(entry.path(), ec);
        }
    }
}

}  // namespace SCE::Test::Mesh
