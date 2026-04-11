// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

module github.com/newmassrael/sce-forge-runtime

go 1.22

// Procedure kind runtime types (ProcedureServiceRequest, ProcedureRunResult,
// RunProcedure) live in sce-go-runtime/forge. Generated procedure fixtures
// import the package via its canonical path — the same path product consumers
// use. The replace directive keeps the conformance harness compile-verifying
// against in-tree sources (no go.sum lookup).
require github.com/newmassrael/sce-go-runtime v0.0.0

replace github.com/newmassrael/sce-go-runtime => ../../sce-go-runtime
