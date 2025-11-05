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
#include <format>
#include <memory>
#include <source_location>
#include <string>

namespace RSM {

/**
 * @brief Centralized logging facade with dependency injection support
 *
 * The Logger class provides a unified logging interface for RSM.
 * It supports two usage patterns:
 *
 * 1. Default mode: Uses built-in backend (spdlog if available, DefaultBackend otherwise)
 * 2. Custom mode: Users inject their own ILoggerBackend implementation
 *
 * Thread-safe: All operations are thread-safe via backend implementation.
 *
 * Example: Using default logger
 * @code
 * RSM::Logger::initialize();
 * LOG_INFO("State machine started");
 * @endcode
 *
 * Example: Injecting custom logger
 * @code
 * RSM::Logger::setBackend(std::make_unique<MyCustomLogger>());
 * LOG_INFO("State machine started");  // Uses MyCustomLogger
 * @endcode
 */
class Logger {
public:
    /**
     * @brief Inject custom logger backend
     *
     * Replaces the default backend with user-provided implementation.
     * Must be called before any logging operations.
     *
     * @param backend User's logger backend (ownership transferred)
     */
    static void setBackend(std::unique_ptr<ILoggerBackend> backend);

    /**
     * @brief Initialize default logger (stdout, no file)
     *
     * Creates default backend if no custom backend was injected.
     */
    static void initialize();

    /**
     * @brief Initialize default logger with file output
     *
     * @param logDir Directory for log files
     * @param logToFile Enable file logging
     */
    static void initialize(const std::string &logDir, bool logToFile = true);

    /**
     * @brief Set minimum log level
     *
     * @param level Minimum level to log
     */
    static void setLevel(LogLevel level);

    // Legacy interface - keep for runtime string concatenation
    static void trace(const std::string &message, const std::source_location &loc = std::source_location::current());
    static void debug(const std::string &message, const std::source_location &loc = std::source_location::current());
    static void info(const std::string &message, const std::source_location &loc = std::source_location::current());
    static void warn(const std::string &message, const std::source_location &loc = std::source_location::current());
    static void error(const std::string &message, const std::source_location &loc = std::source_location::current());

    /**
     * @brief Flush log buffers
     */
    static void flush();

private:
    static std::unique_ptr<ILoggerBackend> backend_;
    static void ensureBackend();
    static std::string extractCleanFunctionName(const std::source_location &loc);
};

}  // namespace RSM

// Macro definitions for std::format support with proper source_location capture
// Uses C++20 std::format instead of spdlog's fmt::format
#define LOG_TRACE(...) RSM::Logger::trace(std::format(__VA_ARGS__), std::source_location::current())
#define LOG_DEBUG(...) RSM::Logger::debug(std::format(__VA_ARGS__), std::source_location::current())
#define LOG_INFO(...) RSM::Logger::info(std::format(__VA_ARGS__), std::source_location::current())
#define LOG_WARN(...) RSM::Logger::warn(std::format(__VA_ARGS__), std::source_location::current())
#define LOG_ERROR(...) RSM::Logger::error(std::format(__VA_ARGS__), std::source_location::current())
