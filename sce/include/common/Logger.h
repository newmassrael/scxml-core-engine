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
#include "common/LogSink.h"
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
 * SCE_LOG_* calls reach the Logger through the sink it installs (`sink()`,
 * docs/adr/0060). `initialize`, `setBackend` and `setLevel` install it, and so does
 * every SCE source file that logs, before any of its own code runs
 * (`common/LogToLogger.h`).
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
     * Replaces the default backend with user-provided implementation, and installs
     * this Logger's sink.
     * Must be called before any logging operations.
     *
     * @param backend User's logger backend (ownership transferred)
     */
    static void setBackend(std::unique_ptr<ILoggerBackend> backend);

    /**
     * @brief Initialize default logger (stdout, no file)
     *
     * Creates default backend if no custom backend was injected, and installs this
     * Logger's sink.
     */
    static void initialize();

    /**
     * @brief Initialize default logger with file output
     *
     * Installs this Logger's sink, as `initialize()` does.
     *
     * @param logDir Directory for log files
     * @param logToFile Enable file logging
     */
    static void initialize(const std::string &logDir, bool logToFile = true);

    /**
     * @brief Set minimum log level
     *
     * Installs this Logger's sink, as `initialize()` does.
     *
     * @param level Minimum level to log
     */
    static void setLevel(LogLevel level);

    /**
     * @brief Check if a given log level is enabled
     *
     * The `wanted` of this Logger's sink: SCE_LOG_* asks it before formatting.
     */
    static bool shouldLog(LogLevel level);

    /**
     * @brief The sink through which SCE_LOG_* reaches this Logger
     *
     * `wanted` is `shouldLog`, and `emit` logs the message at its level as
     * `trace` ... `error` do. A program that installed another sink gives this one
     * back with `SCE::installLogSink(&Logger::sink())`.
     */
    static const LogSink &sink() noexcept;

    /**
     * @brief Installs this Logger's sink when no sink is installed yet
     *
     * The `std::ios_base::Init` pattern: `common/LogToLogger.h` defines one of these
     * in every translation unit that includes it, so the sink is installed before
     * any code of that unit runs. A sink the program installed first is kept.
     */
    class SinkInit {
    public:
        SinkInit() noexcept;
    };

    // Logging methods, at the level each names
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
    static void logAt(LogLevel level, const std::string &message, const SCE::source_location &loc);
    static bool sinkWanted(LogLevel level) noexcept;
    static void sinkEmit(LogLevel level, std::string message, const SCE::source_location &where);
    static std::string extractCleanFunctionName(const SCE::source_location &loc);
};

}  // namespace SCE
