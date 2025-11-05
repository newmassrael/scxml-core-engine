// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-RSM-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// This file is part of RSM (Reactive State Machine).
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
// Full terms: https://github.com/newmassrael/reactive-state-machine/blob/main/LICENSE

#pragma once

#include "common/ILoggerBackend.h"
#include <memory>
#include <spdlog/spdlog.h>

namespace RSM {

/**
 * @brief spdlog-based logger backend
 *
 * Used when RSM is built with spdlog support (RSM_USE_SPDLOG=ON, default).
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

}  // namespace RSM
