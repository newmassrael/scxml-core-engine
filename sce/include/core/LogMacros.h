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
//   Individual: $100 cumulative
//   Enterprise: $500 cumulative
//   Contact: https://github.com/newmassrael
//
// Full terms: https://github.com/newmassrael/scxml-core-engine/blob/main/LICENSE

#pragma once

/**
 * @brief The logging macros of SCE's code, header-only code included
 *
 * Each macro has one definition, the same in every translation unit of a program
 * (docs/adr/0060). It asks the program's installed sink whether its level is wanted
 * and, only when it is, formats the message and hands it to the sink. The arguments
 * are evaluated only then. With no sink installed a call costs one relaxed atomic
 * load and a comparison, and nothing is formatted.
 *
 * Which sink that is, is the program's choice (`common/LogSink.h`): `SCE::Logger`
 * installs its own, so a program that links sce_base logs through the configured
 * `ILoggerBackend`, and a program that uses only the header-only tier logs nothing
 * until it installs one.
 *
 * The message is `{}`-substituted by `SCE::detail::formatLog` (`common/LogFormat.h`):
 * each `{}` takes the next argument through its `operator<<`, and `{{` / `}}` are a
 * literal brace.
 *
 * Usage:
 * @code
 * #include "core/LogMacros.h"
 * SCE_LOG_DEBUG("Transition {} -> {}", static_cast<int>(src), static_cast<int>(tgt));
 * @endcode
 */

#include "common/ILoggerBackend.h"
#include "common/LogFormat.h"
#include "common/LogSink.h"
#include "common/SourceLocation.h"

#define SCE_DETAIL_LOG(level, ...)                                                                                     \
    do {                                                                                                               \
        if (::SCE::detail::logWanted(level))                                                                           \
            ::SCE::detail::logEmit(level, ::SCE::detail::formatLog(__VA_ARGS__), ::SCE::source_location::current());   \
    } while (0)

#define SCE_LOG_TRACE(...) SCE_DETAIL_LOG(::SCE::LogLevel::Trace, __VA_ARGS__)
#define SCE_LOG_DEBUG(...) SCE_DETAIL_LOG(::SCE::LogLevel::Debug, __VA_ARGS__)
#define SCE_LOG_INFO(...) SCE_DETAIL_LOG(::SCE::LogLevel::Info, __VA_ARGS__)
#define SCE_LOG_WARN(...) SCE_DETAIL_LOG(::SCE::LogLevel::Warn, __VA_ARGS__)
#define SCE_LOG_ERROR(...) SCE_DETAIL_LOG(::SCE::LogLevel::Error, __VA_ARGS__)
