// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE-VERIFIES: mesh-14.6
//
// SCE_MESH.md §9.5 mesh-rpc correlation scope — adjudication of the
// §14.6 "Same-target constraint" clause.
//
// The spec states that an RpcReply must be received from the SAME
// transport target as the RpcRequest's `<send target>`, and gives as
// its reason that cross-target pairing "would require an RPC routing
// table the engine does not maintain."
//
// That claim was false: `InvokeCorrelation` IS such a table, and it
// keyed on `invoke_id` alone, so a reply from any source retired the
// entry. §14.6 now defines a responder set — by default the invoke's
// own target — and the table checks it before erasing.
//
// This test pins that gate. The client router registers a mesh-rpc
// invoke against `#motor_invoke`, then a reply carrying the same
// `invoke_id` is injected with a `source` that is NOT the invoke's
// target, and is not any machine in the deployment's topology.
//
// Three arms isolate the correlation axis from every other cause:
//
//   1. matching invoke_id, foreign source  → adjudicates the clause
//   2. matching invoke_id, correct source  → control: the injection
//      shape itself is well-formed and the path is live
//   3. novel invoke_id, correct source     → control: correlation is
//      load-bearing, so arm 1 passing cannot be a vacuous "any
//      envelope reaches Ok" artifact
//
// Arms 2 and 3 are what make arm 1 informative. Without arm 3 a green
// arm 1 could mean the correlation table matches everything; without
// arm 2 a red arm 1 could mean the hand-built envelope was malformed.

#include "brake_invoke_sm.h"
#include "brake_invoke_transport.h"
#include "motor_invoke_sm.h"

#include "mesh/MeshEnvelope.h"

#include <cstdio>
#include <optional>
#include <string>

namespace {

using Brake = SCE::Generated::brake_invoke::brake_invoke;
using BrakeState = SCE::Generated::brake_invoke::State;
using Motor = SCE::Generated::motor_invoke::motor_invoke;
namespace PK = SCE::Mesh;

/// One arm's outcome: whether the reply drove brake out of `Computing`.
struct ArmResult {
    bool reached_ok;
    bool dispatch_returned_true;
    /// Did the §16.7 row 14 raise reach the state machine? `computing`
    /// carries an `error.communication` handler targeting `rejected`,
    /// so this observes the raise as a state change rather than
    /// trusting that the call site exists.
    bool reached_rejected;
};

/// Drive brake to `Computing` (arming a mesh-rpc invoke against
/// `#motor_invoke`), then inject a reply built from `source_name` and
/// either the captured invoke_id or a fresh one.
///
/// The peer link swallows the outbound request instead of forwarding it
/// to a motor session: this test is about what the CLIENT router accepts
/// back, so the real server never replies and the only reply in flight
/// is the one built here.
ArmResult run_arm(const std::string &source_name, bool use_captured_invoke_id) {
    Brake brake;
    Motor motor;  // peer engine the router's template contract requires; the
                  // link below swallows the request so motor never replies.
    SCE::Generated::brake_invoke::TransportRouter<Brake, Motor> router({&brake}, motor);

    std::optional<SCE::uuid::Bytes> captured;
    router.linkTo("#motor_invoke", [&captured](const SCE::Mesh::MeshEnvelope &env) {
        if (env.invoke_id) {
            captured = *env.invoke_id;
        }
        return true;
    });

    brake.initialize();
    brake.processEvent(SCE::Generated::brake_invoke::Event::Go);

    if (brake.getCurrentState() != BrakeState::Computing || !captured.has_value()) {
        // Precondition failure — report as "not ok" so the caller's
        // assertion names the arm that broke.
        return {false, false, false};
    }

    SCE::Mesh::MeshEnvelope reply;
    reply.id = SCE::uuid::v7();
    reply.source = source_name;
    reply.type = "service.response.compute_force";
    reply.pattern = PK::PatternKind::RpcReply;
    reply.invoke_id = use_captured_invoke_id ? *captured : SCE::uuid::v7();

    const bool dispatched = router.dispatchToSession(reply, 0);
    brake.step();

    return {brake.getCurrentState() == BrakeState::Ok, dispatched, brake.getCurrentState() == BrakeState::Rejected};
}

int run_test() {
    // ── Arm 2 (control): correct source, matching invoke_id ──────────
    // Establishes that the hand-built reply envelope is well-formed and
    // that the correlation → done.invoke → Ok path is live in this
    // fixture. If this arm fails, arms 1 and 3 carry no information.
    const ArmResult control_ok = run_arm("motor_invoke", /*use_captured_invoke_id=*/true);
    if (!control_ok.reached_ok) {
        std::fprintf(stderr, "FAIL: control arm — a reply from the invoke's own target did not "
                             "reach Ok; the fixture cannot adjudicate anything\n");
        return 1;
    }

    // ── Arm 3 (control): correct source, novel invoke_id ─────────────
    // Establishes that correlation is load-bearing. A green arm 1 is
    // only meaningful if a NON-matching invoke_id is rejected here.
    const ArmResult control_novel = run_arm("motor_invoke", /*use_captured_invoke_id=*/false);
    if (control_novel.reached_ok) {
        std::fprintf(stderr, "FAIL: control arm — a reply carrying an unregistered invoke_id "
                             "reached Ok; correlation is not load-bearing and arm 1 would be vacuous\n");
        return 2;
    }

    // ── Arm 1: the adjudication ──────────────────────────────────────
    // `sce_no_such_machine` appears in no binding, no deploy.yaml
    // machine list, and is not the invoke's target. Under the §14.6
    // same-target clause this reply must not be correlated.
    const ArmResult foreign = run_arm("sce_no_such_machine", /*use_captured_invoke_id=*/true);

    if (!foreign.reached_rejected) {
        std::fprintf(stderr, "FAIL: the foreign-source reply was not correlated, but no\n"
                             "  error.communication reached the state machine either. A silent\n"
                             "  rejection is indistinguishable from a dropped packet — §16.7 row 14\n"
                             "  exists so the author can observe it.\n");
        return 4;
    }

    if (foreign.reached_ok) {
        std::fprintf(stderr, "FAIL: a reply whose source is neither the invoke's target nor any machine\n"
                             "  in the topology was correlated and delivered. The §14.6 responder-set\n"
                             "  gate is not enforced on the mesh-rpc path — any peer that learns an\n"
                             "  invoke id can retire another peer's pending request.\n");
        return 3;
    }

    std::printf("SCE Mesh §14.6 mesh-rpc responder set: foreign-source reply rejected "
                "with a row-14 raise, declared responder still delivers: PASS\n");
    return 0;
}

}  // namespace

int main() {
    try {
        return run_test();
    } catch (const std::exception &ex) {
        std::fprintf(stderr, "FAIL: unexpected exception: %s\n", ex.what());
        return 1;
    } catch (...) {
        std::fprintf(stderr, "FAIL: unknown exception\n");
        return 1;
    }
}
