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

#include "common/SourceLocation.h"
#include <string>

namespace SCE {

/**
 * @brief Log level enumeration
 *
 * Matches common logging frameworks (spdlog, glog, etc.)
 */
enum class LogLevel { Trace = 0, Debug = 1, Info = 2, Warn = 3, Error = 4, Critical = 5, Off = 6 };

/**
 * @brief Logger backend interface for dependency injection
 *
 * Users can implement this interface to integrate custom logging systems.
 * This allows SCE to use any logging framework (spdlog, glog, custom, etc.)
 * without compile-time dependencies.
 *
 * Example: Custom logger integration
 * @code
 * class MyCompanyLogger : public SCE::ILoggerBackend {
 * public:
 *     void log(LogLevel level, const std::string &message,
 *              const SCE::source_location &loc) override {
 *         myCompanyLoggingSystem->write(level, message, loc.file_name(), loc.line());
 *     }
 *
 *     void setLevel(LogLevel level) override {
 *         myCompanyLoggingSystem->setMinLevel(level);
 *     }
 *
 *     void flush() override {
 *         myCompanyLoggingSystem->flush();
 *     }
 * };
 *
 * // In main():
 * SCE::Logger::setBackend(std::make_unique<MyCompanyLogger>());
 * @endcode
 */
class ILoggerBackend {
public:
    virtual ~ILoggerBackend() = default;

    /**
     * @brief Log a message with source location
     *
     * @param level Log level
     * @param message Pre-formatted message (function name already included)
     * @param loc Source location (file, line, function)
     */
    virtual void log(LogLevel level, const std::string &message, const SCE::source_location &loc) = 0;

    /**
     * @brief Set minimum log level
     *
     * Messages below this level should be ignored.
     */
    virtual void setLevel(LogLevel level) = 0;

    /**
     * @brief Check if a given log level is enabled
     *
     * Used by LOG_* macros to skip std::format evaluation when the level is disabled.
     *
     * @param level Log level to check
     * @return true if messages at this level would be logged
     */
    virtual bool shouldLog(LogLevel level) const = 0;

    /**
     * @brief Flush log buffers
     *
     * Ensures all pending log messages are written.
     */
    virtual void flush() = 0;

    /**
     * @brief Start capturing log messages to an internal buffer
     *
     * Captured logs are stored independently of the current log level,
     * always at debug level. Used for saving diagnostic logs on test failure.
     */
    virtual void startCapture() {}

    /**
     * @brief Stop capturing log messages
     */
    virtual void stopCapture() {}

    /**
     * @brief Retrieve captured log messages and clear the buffer
     *
     * @return All log messages captured since the last startCapture() call
     */
    virtual std::string getCapturedLogs() { return {}; }
};

}  // namespace SCE
