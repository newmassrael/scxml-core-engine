// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
#pragma once

#include <filesystem>
#include <string>
#include <string_view>
#include <system_error>
#include <vector>

namespace SCE::parsing {

// Single source of truth for `<xi:include>` / `<sce:use>` fragment
// path resolution, shared by `XIncludeExpander` and
// `TemplateExpander` so the two preprocessors cannot drift apart.
// Mirrors `sce-build/src/resolve.rs::resolve_fragment`: absolute →
// including-file base directory → operator-configured include
// directories (in declaration order) → current working directory. The
// two implementations are kept byte-equivalent by the template-parity
// harness (`tests/w3c_template_parity/`).
//
// `includeDirs` is the `--include-dir` search path. On a hit, returns
// the resolved path; on a miss, returns an empty path and appends every
// candidate tried (in order) to `searched` so the caller can render its
// own NotFound diagnostic trail (XInclude and template expansion emit
// different diagnostic codes for the same physical miss).
inline std::filesystem::path resolveFragment(std::string_view name, const std::filesystem::path &baseDir,
                                             const std::vector<std::string> &includeDirs,
                                             std::vector<std::string> &searched) {
    const std::filesystem::path path{std::string{name}};
    // Non-throwing `exists` (error_code overload) so an unreadable
    // parent directory yields "not found" rather than a thrown
    // `filesystem_error` — matching Rust `Path::exists()`, which
    // returns false on any error.
    std::error_code ec;

    if (path.is_absolute()) {
        if (std::filesystem::exists(path, ec)) {
            return path;
        }
        searched.push_back(path.string());
        return {};
    }

    if (!baseDir.empty()) {
        const auto candidate = baseDir / path;
        if (std::filesystem::exists(candidate, ec)) {
            return candidate;
        }
        searched.push_back(candidate.string());
    }
    for (const auto &dir : includeDirs) {
        const auto candidate = std::filesystem::path(dir) / path;
        if (std::filesystem::exists(candidate, ec)) {
            return candidate;
        }
        searched.push_back(candidate.string());
    }
    if (std::filesystem::exists(path, ec)) {
        return path;
    }
    searched.push_back(path.string());
    return {};
}

}  // namespace SCE::parsing
