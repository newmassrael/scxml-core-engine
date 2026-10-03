// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package com.sce.runtime

/** The JVM's monitor: reentrant, and held only while [block] runs. */
internal actual fun <T> withMacrostepLock(lock: Any, block: () -> T): T = synchronized(lock, block)
