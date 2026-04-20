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
//   Individual: $100 cumulative
//   Enterprise: $500 cumulative
//   Contact: https://github.com/newmassrael
//
// Full terms: https://github.com/newmassrael/scxml-core-engine/blob/main/LICENSE

#pragma once

/**
 * @brief Conditional logging macros for header-only code
 *
 * When SCE_ENABLE_RUNTIME_LOGGING is defined (by sce_runtime),
 * SCE_LOG_* macros delegate to the full Logger backend via Logger.h.
 *
 * When SCE_ENABLE_RUNTIME_LOGGING is NOT defined (sce_core standalone),
 * SCE_LOG_* macros are no-ops with zero overhead — no std::format evaluation,
 * no Logger.cpp linkage required.
 *
 * Usage in header-only code:
 * @code
 * #include "core/LogMacros.h"
 * SCE_LOG_DEBUG("Transition {} -> {}", static_cast<int>(src), static_cast<int>(tgt));
 * @endcode
 */

#ifdef SCE_ENABLE_RUNTIME_LOGGING
  #include "common/Logger.h"
  #include "common/SourceLocation.h"

  // Format string handling: prefer std::format (C++20), fall back to fmt (spdlog bundled)
  #if __cpp_lib_format >= 201907L
    #include <format>
    #define SCE_DETAIL_FORMAT(...) std::format(__VA_ARGS__)
  #elif defined(SCE_USE_SPDLOG)
    #include <spdlog/fmt/bundled/format.h>
    #define SCE_DETAIL_FORMAT(...) fmt::format(__VA_ARGS__)
  #else
    // No format library available — log format string only (parameter substitution lost)
    #include <string>
    namespace SCE::detail {
    inline std::string format_fallback(const char* fmt) { return std::string(fmt); }
    template<typename... Args>
    inline std::string format_fallback(const char* fmt, Args&&...) { return std::string(fmt); }
    }
    #define SCE_DETAIL_FORMAT(...) SCE::detail::format_fallback(__VA_ARGS__)
  #endif

  #define SCE_LOG_TRACE(...) do { if (SCE::Logger::shouldLog(SCE::LogLevel::Trace)) \
      SCE::Logger::trace(SCE_DETAIL_FORMAT(__VA_ARGS__), SCE::source_location::current()); } while(0)
  #define SCE_LOG_DEBUG(...) do { if (SCE::Logger::shouldLog(SCE::LogLevel::Debug)) \
      SCE::Logger::debug(SCE_DETAIL_FORMAT(__VA_ARGS__), SCE::source_location::current()); } while(0)
  #define SCE_LOG_INFO(...) do { if (SCE::Logger::shouldLog(SCE::LogLevel::Info)) \
      SCE::Logger::info(SCE_DETAIL_FORMAT(__VA_ARGS__), SCE::source_location::current()); } while(0)
  #define SCE_LOG_WARN(...) do { if (SCE::Logger::shouldLog(SCE::LogLevel::Warn)) \
      SCE::Logger::warn(SCE_DETAIL_FORMAT(__VA_ARGS__), SCE::source_location::current()); } while(0)
  #define SCE_LOG_ERROR(...) do { if (SCE::Logger::shouldLog(SCE::LogLevel::Error)) \
      SCE::Logger::error(SCE_DETAIL_FORMAT(__VA_ARGS__), SCE::source_location::current()); } while(0)
#else
  // No-op branch. Arguments are forwarded to a variadic sink so callers do
  // not trip -Wunused-parameter when a logged variable would otherwise be
  // unreferenced. The compiler discards the call entirely at -O1+.
  namespace SCE::detail {
  template <typename... Args>
  inline void log_noop(Args &&...) noexcept {}
  }  // namespace SCE::detail
  #define SCE_LOG_TRACE(...) ::SCE::detail::log_noop(__VA_ARGS__)
  #define SCE_LOG_DEBUG(...) ::SCE::detail::log_noop(__VA_ARGS__)
  #define SCE_LOG_INFO(...)  ::SCE::detail::log_noop(__VA_ARGS__)
  #define SCE_LOG_WARN(...)  ::SCE::detail::log_noop(__VA_ARGS__)
  #define SCE_LOG_ERROR(...) ::SCE::detail::log_noop(__VA_ARGS__)
#endif
