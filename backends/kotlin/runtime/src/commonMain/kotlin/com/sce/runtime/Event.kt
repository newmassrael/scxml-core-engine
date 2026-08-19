// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Kotlin Runtime — Event marker interface

package com.sce.runtime

/**
 * Marker interface for generated event sealed hierarchies.
 *
 * §scxml-3.12.1: Events use sealed interface hierarchy to represent
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

/**
 * §scxml-3.12.2: whether [eventName] names an error the processor itself
 * raised, as opposed to an event the document asked for.
 *
 * The clause reserves the whole `error.` prefix for them: it defines
 * `error.execution` and `error.communication`, lets a platform add a suffix to
 * either, and reserves `error.platform` with or without a suffix on top of
 * that. The prefix is therefore the test — an enumeration would be wrong the
 * first time the set is extended, which the same paragraph says may happen.
 *
 * Used by the engine's internal-queue drain to tell an error nobody answered
 * from an author's own unmatched `<raise>`. The two are indistinguishable in
 * the queue and are not the same event to a host: the author wrote one and can
 * read its fate in the document, while the other was written by the engine to
 * report that the document did not do what it said.
 */
fun isErrorEvent(eventName: String): Boolean = eventName.startsWith("error.")
