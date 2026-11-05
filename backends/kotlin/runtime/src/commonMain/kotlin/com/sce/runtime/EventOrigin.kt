// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The reply address an event carries (docs/adr/0090, section 1): the Event I/O
// Processor that delivered it and the address that processor accepts as a
// `<send>` target. The document reads the two as `_event.origintype` and
// `_event.origin`; they are one value because the address means something only
// relative to its processor. The Kotlin twin of the C++ `SCE::Core::EventOrigin`.

package com.sce.runtime

/**
 * Where a reply to an event goes: the processor and the address it accepts.
 *
 * The processor that delivers an event writes the pair once, where it builds
 * the event (docs/adr/0090, section 2), and nothing converts it later. It is
 * constructed whole: both fields are required, so a pair with one field set
 * cannot be spelled, and an event with no origin holds `null` instead
 * ([EventMetadata.origin]).
 *
 * @property processor The canonical type URI of the Event I/O Processor that
 *   delivered the event (`_event.origintype`).
 * @property address A URI that processor accepts as a `<send>` target
 *   (`_event.origin`).
 */
data class EventOrigin(val processor: String, val address: String)
