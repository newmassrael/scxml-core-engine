// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// Backward-compatibility shim: the procedure runtime types and execution
// engine previously lived here but now have a single home under
// github.com/newmassrael/sce-forge-runtime/forge. This file re-exports
// them via type aliases so that existing imports of
// github.com/newmassrael/sce-go-runtime/forge keep compiling.
//
// New code should prefer importing from sce-forge-runtime/forge directly
// — that is the path the SCE Forge code generator now emits.

package forge

import (
	runtime "github.com/newmassrael/sce-forge-runtime/forge"
)

// Service types re-exported as aliases so interface implementations and
// struct literals on either side remain assignment-compatible.
type (
	ProcedureServiceRequest  = runtime.ProcedureServiceRequest
	ProcedureServiceResponse = runtime.ProcedureServiceResponse
	ProcedureRunResult       = runtime.ProcedureRunResult
	ServiceHandler           = runtime.ServiceHandler
	ProcedurePolicy          = runtime.ProcedurePolicy
)

// RunProcedure forwards to the canonical implementation.
func RunProcedure(policy ProcedurePolicy) ProcedureRunResult {
	return runtime.RunProcedure(policy)
}
