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

#include <source_location>
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
 * This allows RSM to use any logging framework (spdlog, glog, custom, etc.)
 * without compile-time dependencies.
 *
 * Example: Custom logger integration
 * @code
 * class MyCompanyLogger : public SCE::ILoggerBackend {
 * public:
 *     void log(LogLevel level, const std::string &message,
 *              const std::source_location &loc) override {
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
    virtual void log(LogLevel level, const std::string &message, const std::source_location &loc) = 0;

    /**
     * @brief Set minimum log level
     *
     * Messages below this level should be ignored.
     */
    virtual void setLevel(LogLevel level) = 0;

    /**
     * @brief Flush log buffers
     *
     * Ensures all pending log messages are written.
     */
    virtual void flush() = 0;
};

}  // namespace SCE
