// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package com.sce.runtime

/**
 * Run [block] holding [lock], which is the monitor of one engine: a coroutine-mode
 * engine's macrosteps and the host's `stop()` / `cleanup()` of that engine are
 * never inside it at once.
 *
 * The engine's state (its configuration, its queues, its delayed sends) is
 * confined to the one coroutine that runs its macrosteps, and none of it is
 * thread-safe. `stop()` is called from the host's thread, and cancelling the
 * coroutine does not interrupt a macrostep that is already running, so without
 * this the host tore the state down beside the thread still using it.
 *
 * Reentrant, because a handler the engine calls from inside a macrostep may
 * itself stop the engine it was called by. The block never suspends: a monitor
 * cannot be held across a suspension point, and the macrostep it guards does not
 * contain one.
 *
 * A function, not a lock class, because the multiplatform source sets leave the
 * lock to the platform and `expect` / `actual` classes are still Beta, which this
 * module's warnings-as-errors build refuses.
 */
internal expect fun <T> withMacrostepLock(lock: Any, block: () -> T): T
