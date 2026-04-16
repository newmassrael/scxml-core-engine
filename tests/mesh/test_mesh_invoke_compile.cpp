// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh §9.5 invoke plumbing compile verification.
//
// Exercises a fixture where brake hosts an <invoke type="sce:mesh-rpc">.
// Compilation alone catches a class of silent regressions that the existing
// fire-and-forget byte goldens cannot — if `invoke_sites` stops reaching
// the transport template, the generated router will lack invokeMeshRpc /
// cancelMeshRpc entirely, the ctor's setMeshInvokeCallback lambda will not
// be emitted, and the pointer-to-member-function captures below will fail
// to resolve.
//
// The router ctor is instantiated (not just `sizeof`-probed) so the ctor's
// init-list is actually synthesized — which is what caught the `-Wreorder`
// defect the sibling fixtures' `static_assert(sizeof...)` had masked. No
// runtime invoke / cancel is driven here; taking the method pointers is
// enough to force overload resolution without arming the deadline timer
// thread.

#include "brake_invoke_sm.h"
#include "motor_invoke_sm.h"
#include "brake_invoke_transport.h"

#include <cstdio>

int main() {
    SCE::Generated::brake_invoke::brake_invoke brake;
    SCE::Generated::motor_invoke::motor_invoke motor;

    using Router = SCE::Generated::brake_invoke::TransportRouter<
        decltype(brake), decltype(motor)>;
    Router router(brake, motor);

    // Force overload resolution on the two methods so a regression that
    // drops them from the template surfaces as "no member named
    // invokeMeshRpc" here rather than sliding through as a silent no-op.
    constexpr auto invoke_fn = &Router::invokeMeshRpc;
    constexpr auto cancel_fn = &Router::cancelMeshRpc;
    (void)invoke_fn;
    (void)cancel_fn;

    // SCE_MESH.md §9.5 — linkTo plumbing for cross-router correlation
    // handoff. Same overload-resolution forcing so a template
    // regression surfaces at this TU, not silently at the call site.
    constexpr auto link_fn = &Router::linkTo;
    (void)link_fn;

    std::printf("SCE Mesh §9.5 invoke plumbing compile verification: PASS\n");
    return 0;
}
