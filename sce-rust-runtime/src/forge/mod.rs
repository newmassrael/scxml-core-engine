// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// Backward-compatibility shim: the procedure runtime types and execution
// engine previously lived here but now have a single home under
// `sce_forge_runtime::procedure`. This module re-exports them so that
// existing consumers of `sce_rust_runtime::forge::*` keep compiling.
//
// New code should prefer importing from `sce_forge_runtime::procedure`
// directly — that is the path the SCE Forge code generator now emits.

pub use sce_forge_runtime::procedure::{
    run_procedure, ProcedurePolicy, ProcedureRunResult, ProcedureServiceRequest,
    ProcedureServiceResponse,
};
