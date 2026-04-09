// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SCE Forge: Procedure service types for event-driven (Level 2) procedures.

package com.sce.runtime.forge

/** Request sent to a service handler during procedure execution. */
class ProcedureServiceRequest(
    val service: String = "",
    val subfunc: String = "",
) {
    /** Mutable parameters (addr, payload, etc.) — not part of constructor to avoid shallow-copy issues. */
    val params: MutableMap<String, String> = mutableMapOf()
}

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
