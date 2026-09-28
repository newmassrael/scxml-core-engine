// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// This file is part of SCE (SCXML Core Engine).
//
// Dual Licensed:
// 1. AGPL-3.0 + Linking Exception: Free for unmodified use (see LICENSE-EXCEPTION.md)
// 2. Commercial: For modifications (contact newmassrael@gmail.com)
//
// Commercial License:
//   Pricing: contact newmassrael@gmail.com
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

    void log(LogLevel level, const std::string &message, const SCE::source_location &loc) override;
    void setLevel(LogLevel level) override;
    bool shouldLog(LogLevel level) const override;
    void flush() override;

private:
    LogLevel currentLevel_;
    std::mutex mutex_;

    const char *levelToString(LogLevel level);
    const char *levelToColor(LogLevel level);
    std::string getTimestamp();
};

}  // namespace SCE
