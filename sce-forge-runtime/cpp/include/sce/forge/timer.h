// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

// sce_forge_runtime — timer (HAL pattern)
// Pure-virtual interface that the user implements once per platform
// (POSIX, FreeRTOS, Zephyr, std::thread, ...). Generated code holds an
// ITimer& member, never owns the timer object, and bridges into instance
// methods through static-trampoline callbacks. See SCE_FORGE.md §4.10.

#pragma once

#include <cstdint>

namespace SCE::Forge {

/// C-style callback signature. Avoids std::function (heap allocation) and
/// maps 1:1 to native RTOS timer APIs.
using TimerCallback = void (*)(void *ctx);

/// Implementations must be re-entrant safe: a previously scheduled callback
/// must not run after `cancel()` returns. Timer object lifetime is owned by
/// the user (typically static storage); generated code holds references only.
class ITimer {
public:
    virtual ~ITimer() = default;

    virtual void startPeriodic(std::uint32_t intervalMs, TimerCallback cb, void *ctx) = 0;
    virtual void startOneShot(std::uint32_t delayMs, TimerCallback cb, void *ctx) = 0;
    virtual void cancel() = 0;
};

} // namespace SCE::Forge
