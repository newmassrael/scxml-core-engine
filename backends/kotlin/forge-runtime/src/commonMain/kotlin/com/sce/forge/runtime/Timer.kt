// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

package com.sce.forge.runtime

/**
 * Timer HAL — interface that the user implements once per platform.
 * See SCE_FORGE.md Section 4.10.
 *
 * Kotlin's lambda type `() -> Unit` is the natural callback shape on the JVM
 * (no embedded heap constraint), matching the C++/Rust C-style trampoline
 * pattern in behavioural contract: the user supplies a concrete `Timer`
 * implementation that schedules callbacks against the host platform's clock
 * and event loop.
 */
public interface Timer {
    public fun startPeriodic(intervalMs: Long, callback: () -> Unit)
    public fun startOneShot(delayMs: Long, callback: () -> Unit)
    public fun cancel()
}
