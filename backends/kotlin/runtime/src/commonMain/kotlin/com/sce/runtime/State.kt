// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Kotlin Runtime — State marker interface

package com.sce.runtime

/**
 * Marker interface for generated state sealed hierarchies.
 *
 * §scxml-3.2: Each state machine defines a sealed interface
 * extending this marker. Concrete states are `data object` singletons
 * (zero allocation) or `data class` for parallel region composites.
 *
 * Usage in generated code:
 * ```kotlin
 * sealed interface PlayerState : State {
 *     data object Stopped : PlayerState
 *     data object Playing : PlayerState
 * }
 * ```
 */
interface State
