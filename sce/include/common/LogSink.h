// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE::LogSink — where the SCE_LOG_* calls of every translation unit of a program go
// (docs/adr/0060).
//
// Header-only code logs, and it is compiled in SCE's translation units and in a
// consumer's alike, so the call must mean one thing in all of them. It does: it asks
// the program's one sink whether a level is wanted, and only then formats the message
// and hands it over. The sink is chosen by the program at run time, never by how a
// translation unit was compiled. With no sink installed, nothing is wanted and nothing
// is formatted, which is what a program that uses only the header-only tier gets.
//
// `SCE::Logger` (sce_base) installs its own sink, which forwards to the configured
// `ILoggerBackend`. A program that wants another one installs it with
// `installLogSink`.
//
// Limit: an inline variable is one object per program image. A shared library that
// does not export it holds a copy of its own, so a program made of several such
// libraries installs its sink in each. SCE builds static libraries.

#pragma once

#include "common/ILoggerBackend.h"
#include "common/SourceLocation.h"

#include <atomic>
#include <string>
#include <utility>

namespace SCE {

/**
 * @brief The receiver of SCE_LOG_* messages
 *
 * `wanted` answers, before anything is formatted, whether a message at `level`
 * would be kept. `emit` receives the formatted message and the place it was
 * logged from. Both are set: a sink with either one null is not a sink.
 */
struct LogSink {
    bool (*wanted)(LogLevel level) noexcept;
    void (*emit)(LogLevel level, std::string message, const SCE::source_location &where);
};

namespace detail {

/**
 * @brief The program's one installed sink, or null
 *
 * An inline variable with external linkage is the same object in every translation
 * unit of a program ([basic.def.odr]), and its constexpr constructor makes it
 * constant-initialized, so it is null before any code runs and a sink installed by
 * one translation unit is the one every other translation unit reads.
 */
inline std::atomic<const LogSink *> g_logSink{nullptr};

}  // namespace detail

/**
 * @brief Make `sink` the program's sink; null makes it log nothing
 *
 * `sink` must outlive every log call, and it is read without a fence: it must have
 * static storage duration and be constant-initialized, as a `static const` or
 * `constexpr` `LogSink` of two functions is. Its initialization then happens before
 * any thread can read it.
 */
inline void installLogSink(const LogSink *sink) noexcept {
    detail::g_logSink.store(sink, std::memory_order_relaxed);
}

namespace detail {

/**
 * @brief Whether the installed sink keeps a message at `level`; false with none
 *
 * One relaxed load and a comparison when no sink is installed: the whole cost of a
 * log call in a program that installed none.
 */
inline bool logWanted(LogLevel level) noexcept {
    const LogSink *sink = g_logSink.load(std::memory_order_relaxed);
    return sink != nullptr && sink->wanted(level);
}

/**
 * @brief Hand a formatted message to the installed sink, if one still is
 *
 * The sink is read again rather than carried over from `logWanted`: a program may
 * remove it between the two, and then the message has nowhere to go.
 */
inline void logEmit(LogLevel level, std::string message, const SCE::source_location &where) {
    const LogSink *sink = g_logSink.load(std::memory_order_relaxed);
    if (sink != nullptr) {
        sink->emit(level, std::move(message), where);
    }
}

}  // namespace detail

}  // namespace SCE
