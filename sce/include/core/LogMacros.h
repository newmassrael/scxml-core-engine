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
  #include <format>
  #include <source_location>
  #define SCE_LOG_TRACE(...) do { if (SCE::Logger::shouldLog(SCE::LogLevel::Trace)) \
      SCE::Logger::trace(std::format(__VA_ARGS__), std::source_location::current()); } while(0)
  #define SCE_LOG_DEBUG(...) do { if (SCE::Logger::shouldLog(SCE::LogLevel::Debug)) \
      SCE::Logger::debug(std::format(__VA_ARGS__), std::source_location::current()); } while(0)
  #define SCE_LOG_INFO(...) do { if (SCE::Logger::shouldLog(SCE::LogLevel::Info)) \
      SCE::Logger::info(std::format(__VA_ARGS__), std::source_location::current()); } while(0)
  #define SCE_LOG_WARN(...) do { if (SCE::Logger::shouldLog(SCE::LogLevel::Warn)) \
      SCE::Logger::warn(std::format(__VA_ARGS__), std::source_location::current()); } while(0)
  #define SCE_LOG_ERROR(...) do { if (SCE::Logger::shouldLog(SCE::LogLevel::Error)) \
      SCE::Logger::error(std::format(__VA_ARGS__), std::source_location::current()); } while(0)
#else
  #define SCE_LOG_TRACE(...) ((void)0)
  #define SCE_LOG_DEBUG(...) ((void)0)
  #define SCE_LOG_INFO(...)  ((void)0)
  #define SCE_LOG_WARN(...)  ((void)0)
  #define SCE_LOG_ERROR(...) ((void)0)
#endif
