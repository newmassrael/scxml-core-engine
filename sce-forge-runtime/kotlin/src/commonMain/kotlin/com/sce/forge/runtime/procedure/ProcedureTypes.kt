// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SCE Forge: Procedure service types for event-driven (Level 2) procedures.

package com.sce.forge.runtime.procedure

/**
 * Request sent to a service handler during procedure execution.
 *
 * Fields map 1:1 to the four `<send>` attributes in the SCXML source:
 *
 * ```
 * <send sce:service="Diag" sce:subfunc="0x02"
 *       sce:addr="ecuAddr" sce:payload="frame.encode()"/>
 * ```
 *
 * `service` is always present; the other three use nullable types so that
 * absent attributes are distinguishable from empty values. `payload` is
 * typed as `ByteArray` because its semantic role is a raw wire-format data
 * blob originating from codec `encode()` calls. `subfunc` and `addr`
 * remain textual since the user may reference datamodel variables of any
 * SCE type.
 */
// Uses `class` rather than `data class` because Kotlin's generated equals()
// for a data class with a ByteArray field uses reference equality on the
// array, which would silently break test comparisons. We do not rely on
// structural equality for ProcedureServiceRequest today; if a future test
// needs it, override equals/hashCode to call contentEquals on payload.
class ProcedureServiceRequest(
    val service: String,
    val subfunc: String? = null,
    val addr: String? = null,
    val payload: ByteArray? = null,
)

/** Response received from a service handler. */
data class ProcedureServiceResponse(
    val success: Boolean = false,
    val data: String = ""
)

/** Result of running a procedure to completion. */
data class ProcedureRunResult(
    val completed: Boolean = false,
    val finalState: String = "",
    val doneData: Map<String, String> = emptyMap()
)

/** Service handler callback type for procedure dispatch. */
typealias ProcedureServiceHandler = (ProcedureServiceRequest) -> ProcedureServiceResponse
