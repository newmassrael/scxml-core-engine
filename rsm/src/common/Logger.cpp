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

#include "common/Logger.h"

#ifdef RSM_USE_SPDLOG
#include "backends/SpdlogBackend.h"
#else
#include "backends/DefaultBackend.h"
#endif

#include <algorithm>
#include <mutex>

namespace RSM {

// Static member initialization
std::unique_ptr<ILoggerBackend> Logger::backend_;

// Mutex for thread-safe backend initialization
static std::mutex backend_mutex;

void Logger::setBackend(std::unique_ptr<ILoggerBackend> backend) {
    std::lock_guard<std::mutex> lock(backend_mutex);
    backend_ = std::move(backend);
}

void Logger::initialize() {
    std::lock_guard<std::mutex> lock(backend_mutex);
    if (!backend_) {
#ifdef RSM_USE_SPDLOG
        backend_ = std::make_unique<SpdlogBackend>();
#else
        backend_ = std::make_unique<DefaultBackend>();
#endif
    }
}

void Logger::initialize([[maybe_unused]] const std::string &logDir, [[maybe_unused]] bool logToFile) {
    std::lock_guard<std::mutex> lock(backend_mutex);
    if (!backend_) {
#ifdef RSM_USE_SPDLOG
        backend_ = std::make_unique<SpdlogBackend>(logDir, logToFile);
#else
        // DefaultBackend doesn't support file logging
        backend_ = std::make_unique<DefaultBackend>();
#endif
    }
}

void Logger::setLevel(LogLevel level) {
    ensureBackend();
    backend_->setLevel(level);
}

void Logger::trace(const std::string &message, const std::source_location &loc) {
    ensureBackend();
    std::string enhanced_message = extractCleanFunctionName(loc) + "() - " + message;
    backend_->log(LogLevel::Trace, enhanced_message, loc);
}

void Logger::debug(const std::string &message, const std::source_location &loc) {
    ensureBackend();
    std::string enhanced_message = extractCleanFunctionName(loc) + "() - " + message;
    backend_->log(LogLevel::Debug, enhanced_message, loc);
}

void Logger::info(const std::string &message, const std::source_location &loc) {
    ensureBackend();
    std::string enhanced_message = extractCleanFunctionName(loc) + "() - " + message;
    backend_->log(LogLevel::Info, enhanced_message, loc);
}

void Logger::warn(const std::string &message, const std::source_location &loc) {
    ensureBackend();
    std::string enhanced_message = extractCleanFunctionName(loc) + "() - " + message;
    backend_->log(LogLevel::Warn, enhanced_message, loc);
}

void Logger::error(const std::string &message, const std::source_location &loc) {
    ensureBackend();
    std::string enhanced_message = extractCleanFunctionName(loc) + "() - " + message;
    backend_->log(LogLevel::Error, enhanced_message, loc);
}

void Logger::flush() {
    ensureBackend();
    backend_->flush();
}

void Logger::ensureBackend() {
    if (!backend_) {
        initialize();
    }
}

std::string Logger::extractCleanFunctionName(const std::source_location &loc) {
    std::string full_name = loc.function_name();

    // Find the opening parenthesis
    size_t paren_pos = full_name.find('(');
    if (paren_pos == std::string::npos) {
        return "UnknownFunction";
    }

    // Work backwards from the opening parenthesis
    size_t name_end = paren_pos;
    while (name_end > 0 && (std::isspace(full_name[name_end - 1]) || full_name[name_end - 1] == ')')) {
        name_end--;
    }

    // Find the start of the qualified function name
    size_t name_start = 0;
    size_t space_pos = std::string::npos;

    // Find the last space that's not inside template parameters
    int angle_bracket_count = 0;
    int paren_count = 0;
    for (size_t i = 0; i < name_end; i++) {
        char c = full_name[i];
        if (c == '<') {
            angle_bracket_count++;
        } else if (c == '>') {
            angle_bracket_count--;
        } else if (c == '(') {
            paren_count++;
        } else if (c == ')') {
            paren_count--;
        } else if (c == ' ' && angle_bracket_count == 0 && paren_count == 0) {
            space_pos = i;
        }
    }

    if (space_pos != std::string::npos) {
        name_start = space_pos + 1;
    }

    // Extract the qualified function name
    std::string qualified_name = full_name.substr(name_start, name_end - name_start);

    // Clean up any remaining artifacts
    while (!qualified_name.empty() &&
           (std::isspace(qualified_name[0]) || qualified_name[0] == '*' || qualified_name[0] == '&')) {
        qualified_name = qualified_name.substr(1);
    }

    // Remove template parameters if they exist
    std::string result;
    int angle_count = 0;
    for (char c : qualified_name) {
        if (c == '<') {
            angle_count++;
        } else if (c == '>') {
            angle_count--;
        } else if (angle_count == 0) {
            result += c;
        }
    }

    // Remove trailing whitespace
    while (!result.empty() && std::isspace(result.back())) {
        result.pop_back();
    }

    return result.empty() ? "UnknownFunction" : result;
}

}  // namespace RSM
