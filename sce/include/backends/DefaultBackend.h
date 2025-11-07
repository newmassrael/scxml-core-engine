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
#include <chrono>
#include <iostream>
#include <mutex>

namespace SCE {

/**
 * @brief Simple stdout logger with no external dependencies
 *
 * Used when SCE is built without spdlog (SCE_USE_SPDLOG=OFF).
 * Provides basic logging to stdout with:
 * - Thread-safe output (std::mutex)
 * - Timestamp (HH:MM:SS.mmm)
 * - Log level coloring (ANSI codes)
 * - Source location (file:line)
 *
 * No advanced features:
 * - No file logging
 * - No log rotation
 * - No custom formatters
 *
 * For production use, inject custom ILoggerBackend implementation.
 */
class DefaultBackend : public ILoggerBackend {
public:
    DefaultBackend();

    void log(LogLevel level, const std::string &message, const std::source_location &loc) override;
    void setLevel(LogLevel level) override;
    void flush() override;

private:
    LogLevel currentLevel_;
    std::mutex mutex_;

    const char *levelToString(LogLevel level);
    const char *levelToColor(LogLevel level);
    std::string getTimestamp();
};

}  // namespace SCE
