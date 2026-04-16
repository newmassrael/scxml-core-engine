// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// This file is part of SCE (SCXML Core Engine).
//
// Dual Licensed:
// 1. LGPL-2.1: Free for unmodified use (see LICENSE-LGPL-2.1.md)
// 2. Commercial: For modifications (contact newmassrael@gmail.com)
//
// Commercial License:
//   Individual: $5000 cumulative
//   Enterprise: Contact for pricing
//   Contact: https://github.com/newmassrael
//
// Full terms: https://github.com/newmassrael/scxml-core-engine/blob/main/LICENSE

#pragma once

#include "common/ILoggerBackend.h"
#include "common/SourceLocation.h"
#include <memory>
#include <string>

namespace SCE {

/**
 * @brief Centralized logging facade with dependency injection support
 *
 * The Logger class provides a unified logging interface for SCE.
 * It supports two usage patterns:
 *
 * 1. Default mode: Uses built-in backend (spdlog if available, DefaultBackend otherwise)
 * 2. Custom mode: Users inject their own ILoggerBackend implementation
 *
 * Thread-safe: All operations are thread-safe via backend implementation.
 *
 * Example: Using default logger
 * @code
 * SCE::Logger::initialize();
 * SCE_LOG_INFO("State machine started");
 * @endcode
 *
 * Example: Injecting custom logger
 * @code
 * SCE::Logger::setBackend(std::make_unique<MyCustomLogger>());
 * SCE_LOG_INFO("State machine started");  // Uses MyCustomLogger
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

    /**
     * @brief Check if a given log level is enabled
     *
     * Used by SCE_LOG_* macros to avoid std::format evaluation when the level is disabled.
     */
    static bool shouldLog(LogLevel level);

    // Logging methods — called by SCE_LOG_* macros in LogMacros.h
    static void trace(const std::string &message, const SCE::source_location &loc = SCE::source_location::current());
    static void debug(const std::string &message, const SCE::source_location &loc = SCE::source_location::current());
    static void info(const std::string &message, const SCE::source_location &loc = SCE::source_location::current());
    static void warn(const std::string &message, const SCE::source_location &loc = SCE::source_location::current());
    static void error(const std::string &message, const SCE::source_location &loc = SCE::source_location::current());

    /**
     * @brief Flush log buffers
     */
    static void flush();

    /**
     * @brief Start capturing log messages to an internal buffer
     *
     * Captured logs are stored at debug level regardless of the active log level.
     * Used for saving diagnostic output when a test fails.
     */
    static void startCapture();

    /**
     * @brief Stop capturing log messages
     */
    static void stopCapture();

    /**
     * @brief Retrieve and clear captured log messages
     *
     * @return All log messages captured since the last startCapture() call
     */
    static std::string getCapturedLogs();

private:
    static std::unique_ptr<ILoggerBackend> backend_;
    static void ensureBackend();
    static std::string extractCleanFunctionName(const SCE::source_location &loc);
};

}  // namespace SCE
