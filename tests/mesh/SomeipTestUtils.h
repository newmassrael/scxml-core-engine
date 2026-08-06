// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// vsomeip-specific test utilities layered on top of MeshTestUtils.h.

#pragma once

#include <cstdlib>
#include <filesystem>
#include <fstream>
#include <sstream>
#include <string>
#include <system_error>
#include <vector>

namespace SCE::Test::Mesh {

namespace detail {

/// The `network:` value declared by the vsomeip config at `path`, or an
/// empty string.
///
/// A substring scan rather than a JSON parse: this header is included by
/// every SOME/IP fixture and has no JSON dependency to reach for, the
/// configs it reads are repo-owned and one key deep, and the failure mode
/// of a miss is that `wipe_stale_vsomeip_sockets` wipes less — never that
/// it wipes something it should not.
inline std::string vsomeip_network_from_config(const std::filesystem::path &path) {
    std::ifstream in(path);
    if (!in) {
        return {};
    }
    std::stringstream buf;
    buf << in.rdbuf();
    const std::string text = buf.str();
    const auto key = text.find("\"network\"");
    if (key == std::string::npos) {
        return {};
    }
    const auto colon = text.find(':', key);
    if (colon == std::string::npos) {
        return {};
    }
    const auto open_quote = text.find('"', colon);
    if (open_quote == std::string::npos) {
        return {};
    }
    const auto close_quote = text.find('"', open_quote + 1);
    if (close_quote == std::string::npos) {
        return {};
    }
    return text.substr(open_quote + 1, close_quote - open_quote - 1);
}

}  // namespace detail

// Wipe stale vsomeip local endpoints left by crashed/killed prior runs.
//
// What was measured, and what was not:
//
// vsomeip names its local sockets `/tmp/<network>-<client_id>`, where
// `<network>` is the config's own `network:` value and `vsomeip` only when
// none is declared. All 18 SOME/IP fixtures in this tree declare one — so
// each owns a disjoint /tmp namespace and ctest -j can run them together —
// which means the `vsomeip-` prefix this function used to match names
// nothing any of them creates: `ls /tmp | grep -c '^vsomeip-'` is 0 while
// the fixtures are running. The cleanup was a no-op for the whole tree.
//
// It is NOT established that the no-op ever cost anything. Planting a real
// stale unix socket at a fixture's routing path and running it with the
// old prefix list still passes: vsomeip 3.7.3 unlinks and rebinds the
// path itself. So this widening makes the function do what its name says
// rather than fixing an observed failure, and a future reader should not
// infer from its presence that stale sockets are known to break a run.
//
// Scope is deliberately one namespace: a fixture owns its `network:`, so
// wiping only that prefix cannot disturb a sibling running concurrently.
// The active config comes from VSOMEIP_CONFIGURATION, the same variable
// ctest sets per fixture.
//
// Using the filesystem API (not system("rm")) keeps the test hermetic with
// no shell dependency. Errors from missing/already-unlinked files are
// ignored.
inline void wipe_stale_vsomeip_sockets() {
    std::vector<std::string> prefixes{"vsomeip-"};
    if (const char *cfg = std::getenv("VSOMEIP_CONFIGURATION")) {
        const std::string network = detail::vsomeip_network_from_config(cfg);
        if (!network.empty()) {
            prefixes.push_back(network + "-");
        }
    }

    std::error_code ec;
    for (const auto &entry : std::filesystem::directory_iterator("/tmp", ec)) {
        if (ec) {
            return;
        }
        const auto name = entry.path().filename().string();
        for (const auto &prefix : prefixes) {
            if (name.rfind(prefix, 0) == 0) {
                std::filesystem::remove(entry.path(), ec);
                break;
            }
        }
    }
}

}  // namespace SCE::Test::Mesh
