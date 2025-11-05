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

#include "backends/DefaultBackend.h"
#include <algorithm>
#include <cstdlib>
#include <ctime>
#include <format>
#include <iomanip>
#include <sstream>

namespace RSM {

// ANSI color codes for terminal output
namespace Colors {
constexpr const char *RESET = "\033[0m";
constexpr const char *TRACE = "\033[37m";     // White
constexpr const char *DEBUG = "\033[36m";     // Cyan
constexpr const char *INFO = "\033[32m";      // Green
constexpr const char *WARN = "\033[33m";      // Yellow
constexpr const char *ERROR = "\033[31m";     // Red
constexpr const char *CRITICAL = "\033[35m";  // Magenta
}  // namespace Colors

DefaultBackend::DefaultBackend() : currentLevel_(LogLevel::Debug) {
    // Check SPDLOG_LEVEL environment variable for compatibility
    const char *env_level = std::getenv("SPDLOG_LEVEL");
    if (env_level) {
        std::string level_str(env_level);
        std::transform(level_str.begin(), level_str.end(), level_str.begin(), ::tolower);

        if (level_str == "trace") {
            currentLevel_ = LogLevel::Trace;
        } else if (level_str == "debug") {
            currentLevel_ = LogLevel::Debug;
        } else if (level_str == "info") {
            currentLevel_ = LogLevel::Info;
        } else if (level_str == "warn" || level_str == "warning") {
            currentLevel_ = LogLevel::Warn;
        } else if (level_str == "err" || level_str == "error") {
            currentLevel_ = LogLevel::Error;
        } else if (level_str == "critical") {
            currentLevel_ = LogLevel::Critical;
        } else if (level_str == "off") {
            currentLevel_ = LogLevel::Off;
        }
    }
}

void DefaultBackend::log(LogLevel level, const std::string &message, [[maybe_unused]] const std::source_location &loc) {
    if (level < currentLevel_) {
        return;
    }

    std::lock_guard<std::mutex> lock(mutex_);

    // Format: [HH:MM:SS.mmm] [LEVEL] message
    std::cout << "[" << getTimestamp() << "] "
              << "[" << levelToColor(level) << levelToString(level) << Colors::RESET << "] " << message << "\n";
}

void DefaultBackend::setLevel(LogLevel level) {
    std::lock_guard<std::mutex> lock(mutex_);
    currentLevel_ = level;
}

void DefaultBackend::flush() {
    std::lock_guard<std::mutex> lock(mutex_);
    std::cout.flush();
}

const char *DefaultBackend::levelToString(LogLevel level) {
    switch (level) {
    case LogLevel::Trace:
        return "trace";
    case LogLevel::Debug:
        return "debug";
    case LogLevel::Info:
        return "info";
    case LogLevel::Warn:
        return "warn";
    case LogLevel::Error:
        return "error";
    case LogLevel::Critical:
        return "critical";
    case LogLevel::Off:
        return "off";
    default:
        return "unknown";
    }
}

const char *DefaultBackend::levelToColor(LogLevel level) {
    switch (level) {
    case LogLevel::Trace:
        return Colors::TRACE;
    case LogLevel::Debug:
        return Colors::DEBUG;
    case LogLevel::Info:
        return Colors::INFO;
    case LogLevel::Warn:
        return Colors::WARN;
    case LogLevel::Error:
        return Colors::ERROR;
    case LogLevel::Critical:
        return Colors::CRITICAL;
    default:
        return Colors::RESET;
    }
}

std::string DefaultBackend::getTimestamp() {
    auto now = std::chrono::system_clock::now();
    auto now_time_t = std::chrono::system_clock::to_time_t(now);
    auto now_ms = std::chrono::duration_cast<std::chrono::milliseconds>(now.time_since_epoch()) % 1000;

    std::tm tm_buf;
#ifdef _WIN32
    localtime_s(&tm_buf, &now_time_t);
#else
    localtime_r(&now_time_t, &tm_buf);
#endif

    return std::format("{:02d}:{:02d}:{:02d}.{:03d}", tm_buf.tm_hour, tm_buf.tm_min, tm_buf.tm_sec,
                       static_cast<int>(now_ms.count()));
}

}  // namespace RSM
