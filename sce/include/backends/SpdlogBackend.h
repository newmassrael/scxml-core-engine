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
#include <memory>
#include <mutex>
#include <spdlog/sinks/base_sink.h>
#include <spdlog/spdlog.h>
#include <sstream>

namespace SCE {

/**
 * @brief Custom spdlog sink that captures log messages to an in-memory buffer
 *
 * Used to capture diagnostic logs during test execution. When a test fails,
 * the captured buffer can be saved to a file for post-mortem analysis.
 * Always captures at debug level regardless of the logger's active level.
 */
template <typename Mutex>
class CaptureSink : public spdlog::sinks::base_sink<Mutex> {
public:
    std::string getBuffer() {
        std::lock_guard<Mutex> lock(spdlog::sinks::base_sink<Mutex>::mutex_);
        return buffer_.str();
    }

    void clearBuffer() {
        std::lock_guard<Mutex> lock(spdlog::sinks::base_sink<Mutex>::mutex_);
        buffer_.str("");
        buffer_.clear();
    }

protected:
    void sink_it_(const spdlog::details::log_msg &msg) override {
        spdlog::memory_buf_t formatted;
        spdlog::sinks::base_sink<Mutex>::formatter_->format(msg, formatted);
        buffer_.write(formatted.data(), static_cast<std::streamsize>(formatted.size()));
    }

    void flush_() override {}

private:
    std::ostringstream buffer_;
};

using CaptureSinkMt = CaptureSink<std::mutex>;

/**
 * @brief spdlog-based logger backend
 *
 * Used when SCE is built with spdlog support (SCE_USE_SPDLOG=ON, default).
 * Provides full spdlog features:
 * - Multiple sinks (console, file, etc.)
 * - File rotation
 * - Custom formatters
 * - High performance
 * - Per-test log capture for failure diagnostics
 *
 * This is the default backend when spdlog is available.
 */
class SpdlogBackend : public ILoggerBackend {
public:
    SpdlogBackend(const std::string &logDir = "", bool logToFile = false);

    void log(LogLevel level, const std::string &message, const SCE::source_location &loc) override;
    void setLevel(LogLevel level) override;
    bool shouldLog(LogLevel level) const override;
    void flush() override;

    void startCapture() override;
    void stopCapture() override;
    std::string getCapturedLogs() override;

private:
    std::shared_ptr<spdlog::logger> logger_;
    std::shared_ptr<CaptureSinkMt> captureSink_;
    spdlog::level::level_enum preCaptureLevel_ = spdlog::level::off;
    bool capturing_ = false;

    spdlog::level::level_enum convertLevel(LogLevel level) const;
};

}  // namespace SCE
