// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// This file is part of SCE (SCXML Core Engine).
//
// Dual Licensed:
// 1. LGPL-2.1: Free for unmodified use (see LICENSE-LGPL-2.1.md)
// 2. Commercial: For modifications (contact newmassrael@gmail.com)
//
// Commercial License:
//   Individual: $100 cumulative
//   Enterprise: $500 cumulative
//   Contact: https://github.com/newmassrael
//
// Full terms: https://github.com/newmassrael/scxml-core-engine/blob/main/LICENSE

#pragma once

#include "common/ILoggerBackend.h"
#include <memory>
#include <spdlog/spdlog.h>

namespace SCE {

/**
 * @brief spdlog-based logger backend
 *
 * Used when SCE is built with spdlog support (SCE_USE_SPDLOG=ON, default).
 * Provides full spdlog features:
 * - Multiple sinks (console, file, etc.)
 * - File rotation
 * - Custom formatters
 * - High performance
 *
 * This is the default backend when spdlog is available.
 */
class SpdlogBackend : public ILoggerBackend {
public:
    SpdlogBackend(const std::string &logDir = "", bool logToFile = false);

    void log(LogLevel level, const std::string &message, const std::source_location &loc) override;
    void setLevel(LogLevel level) override;
    void flush() override;

private:
    std::shared_ptr<spdlog::logger> logger_;

    spdlog::level::level_enum convertLevel(LogLevel level);
};

}  // namespace SCE
