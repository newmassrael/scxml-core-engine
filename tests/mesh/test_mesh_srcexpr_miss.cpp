// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh §9.5 srcexpr target miss runtime verification.
//
// Companion to `mesh_srcexpr_compile_verification` (compile-only).
// This fixture exercises the pre-envelope setup-fault raise path
// end-to-end: srcexpr evaluates to `#ghost_target` which has no
// matching deploy.yaml binding, `performMeshInvoke` returns false,
// and the single `error.execution` raise at
// `entry_exit_actions.jinja2` L219-224 fires.
//
// §9.5 L1340 / §10.7.1 pin `error.execution` (not
// `error.communication`; see commit 03d0b6dd retiring §16.7 row 6)
// as the event class for this tier. This fixture is the runtime
// evidence the catalog's `P` classification otherwise lacked.
//
// Raise-class guard: §9.5 L1345-1350 forbids the pre-envelope tier
// from using `error.invoke.<id>` (that tier is reserved for reply-side
// `rpc_status != Ok`). The scxml traps a wrong-class raise via a
// dedicated `invoke_raise_regression` final state reached from either
// `calling_miss` or `observe` on `error.invoke`.
//
// Single-raise guard: the srcexpr branch at entry_exit_actions.jinja2
// L197-223 has three raise points (eval fail, shape fail, miss). The
// contract is exactly one raise per failed invoke; a regression
// reintroducing a second error.execution on this path is trapped by
// `observe`'s transition to `double_raise_detected`.
//
// Scope note: this fixture covers the SRCEXPR miss path. The §14.4
// pool-miss path (the target of commit 35340cc7) flows through a
// different raise site (entry_exit_actions.jinja2 L229-244) and is
// not exercised here. Pool-miss runtime coverage would need a
// vsomeip-backed fixture and is out of scope for the srcexpr
// invariant this test pins.

#include "common/TestScriptEngine.h"
#include "motor_srcexpr_sm.h"
#include "srcexpr_miss_sm.h"
#include "srcexpr_miss_transport.h"

#include <cstdio>

int main() {
    SCE::Generated::srcexpr_miss::srcexpr_miss machine;
    // motor_srcexpr is the phantom binding target — instantiated so the
    // two-engine TransportRouter construction typechecks, never stepped.
    SCE::Generated::motor_srcexpr::motor_srcexpr phantom_peer;

    using Router = SCE::Generated::srcexpr_miss::TransportRouter<decltype(machine), decltype(phantom_peer)>;
    Router router({&machine}, phantom_peer);

    SCE::Test::inject_build_engine(machine);
    machine.initialize();

    // probe.go drives idle → calling_miss. The calling_miss entry
    // evaluates the srcexpr, calls performMeshInvoke on #ghost_target,
    // gets false back, and raises error.execution once. The transition
    // on error.execution at calling_miss routes to observe.
    machine.processEvent(SCE::Generated::srcexpr_miss::Event::Probe_go);

    const auto after_probe = machine.getCurrentState();
    if (after_probe == SCE::Generated::srcexpr_miss::State::Double_raise_detected) {
        std::printf("FAIL: srcexpr miss path raised error.execution twice. The "
                    "single-raise contract at entry_exit_actions.jinja2 L197-223 "
                    "regressed — inspect all three raise points on the srcexpr "
                    "branch for a newly added sibling raise.\n");
        return 2;
    }
    if (after_probe == SCE::Generated::srcexpr_miss::State::Invoke_raise_regression) {
        std::printf("FAIL: srcexpr miss routed through error.invoke.<id>. §9.5 "
                    "L1345-1350 pins the pre-envelope tier to error.execution; "
                    "a reply-tier raise on a pre-envelope fault violates the "
                    "three-tier classification.\n");
        return 3;
    }
    if (after_probe != SCE::Generated::srcexpr_miss::State::Observe) {
        std::printf("FAIL: expected State::Observe after probe.go, got state=%d. "
                    "The §9.5 L1340 error.execution raise path did not fire — "
                    "srcexpr evaluation or performMeshInvoke routing likely regressed.\n",
                    static_cast<int>(after_probe));
        return 4;
    }

    // settle drives observe → handled. No further error.execution is
    // expected; handled is the single-raise pass verdict.
    machine.processEvent(SCE::Generated::srcexpr_miss::Event::Settle);

    if (machine.getCurrentState() != SCE::Generated::srcexpr_miss::State::Handled) {
        std::printf("FAIL: settle did not drive observe → handled (state=%d).\n",
                    static_cast<int>(machine.getCurrentState()));
        return 5;
    }

    std::printf("SCE Mesh §9.5 srcexpr target miss runtime: PASS\n");
    return 0;
}
