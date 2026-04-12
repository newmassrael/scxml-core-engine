// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Kotlin Runtime — Event marker interface

package com.sce.runtime

/**
 * Marker interface for generated event sealed hierarchies.
 *
 * W3C SCXML 3.12.1: Events use sealed interface hierarchy to represent
 * dot-separated event names. Prefix matching becomes Kotlin `is` type checks.
 *
 * Usage in generated code:
 * ```kotlin
 * sealed interface PlayerEvent : Event {
 *     data object Play : PlayerEvent
 *     sealed interface Error : PlayerEvent {
 *         data object Execution : Error   // "error.execution"
 *     }
 * }
 * ```
 *
 * W3C prefix matching: `event is PlayerEvent.Error` matches all error.* events.
 */
interface Event
