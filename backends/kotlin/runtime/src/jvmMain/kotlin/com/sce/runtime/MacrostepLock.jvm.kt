// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package com.sce.runtime

/** The JVM's monitor: reentrant, and held only while [block] runs. */
internal actual fun <T> withMacrostepLock(lock: Any, block: () -> T): T = synchronized(lock, block)

/** Whether this thread holds the monitor of [lock]. */
internal actual fun holdsMacrostepLock(lock: Any): Boolean = Thread.holdsLock(lock)

/** The JVM thread running the caller. */
internal actual fun currentThreadIdentity(): Any = Thread.currentThread()
