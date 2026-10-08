// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE_MESH.md §9.5 mesh-rpc `srcexpr` under `datamodel="sce-static"` (docs/adr/0005,
// decision 7): the peer a Mesh request asks is a string the machine computes from its
// own fields when the invocation starts, looked up among the bindings the deployment
// declares, by the router the build generates for it.
//
// Fixture: tests/mesh/brake_static_srcexpr.scxml, with `motor_static_srcexpr.scxml` as
// the peer the router is parameterised on and `deploy_static_srcexpr.yaml` as the
// deployment. The request is observed where the router hands it to its link, so no
// transport is involved and the peer never runs; the reply the last case needs is built
// by hand, as `test_mesh_static_invoke_request.cpp` builds one.
//
// The machine has no script engine, and the test is linked without one: the link is the
// proof that the peer's name never reached an engine.

#include "brake_static_srcexpr_sm.h"
#include "brake_static_srcexpr_transport.h"
#include "motor_static_srcexpr_sm.h"

#include "mesh/MeshEnvelope.h"

#include <algorithm>
#include <cstdio>
#include <optional>
#include <string>
#include <vector>

namespace {

using Brake = SCE::Generated::brake_static_srcexpr::brake_static_srcexpr;
using BrakeState = SCE::Generated::brake_static_srcexpr::State;
using Motor = SCE::Generated::motor_static_srcexpr::motor_static_srcexpr;
using Router = SCE::Generated::brake_static_srcexpr::TransportRouter<Brake, Motor>;

/// A machine and its router, with the requests the router handed to its link.
struct Harness {
    Brake brake;
    Motor motor;  // the peer the router's template is parameterised on; the link below
                  // swallows the request, so it never runs.
    Router router;
    std::vector<SCE::Mesh::MeshEnvelope> sent;

    Harness() : router({&brake}, motor) {
        router.linkTo("#motor_static_srcexpr", [this](const SCE::Mesh::MeshEnvelope &env) {
            sent.push_back(env);
            return true;
        });
        brake.initialize();
        brake.step();
    }

    void raise(const std::string &event) {
        brake.raiseExternal(event);
        brake.step();
    }
};

int fail(const char *what) {
    std::fprintf(stderr, "FAIL: %s\n", what);
    return 1;
}

/// The payload of the request, without the spaces a writer may put between tokens.
std::string payloadOf(const SCE::Mesh::MeshEnvelope &env) {
    std::string body(env.data.begin(), env.data.end());
    body.erase(std::remove(body.begin(), body.end(), ' '), body.end());
    return body;
}

// `peer` starts as the one binding the deployment declares, so a machine that has changed
// nothing asks it, and the request carries what the machine's other fields hold.
int a_request_goes_to_the_binding_the_peer_field_names() {
    Harness h;
    h.raise("go");
    if (h.sent.size() != 1) {
        return fail("one request is handed to the link of the binding `peer` names");
    }
    if (h.sent[0].type != "service.request.compute_force") {
        return fail("the request carries the `_mesh_event` constant as its type");
    }
    if (payloadOf(h.sent[0]).find("\"load\":3") == std::string::npos) {
        return fail("the request carries the field the machine's param reads");
    }
    if (h.brake.getCurrentState() != BrakeState::Computing) {
        return fail("the machine waits in `computing` for the reply");
    }
    if (h.brake.errors() != 0u) {
        return fail("a request that went raised an error");
    }
    return 0;
}

// A well-formed name that no binding of the deployment carries is a setup fault of the
// pre-envelope tier: `error.execution` once, and nothing reaches the link.
int a_peer_that_matches_no_binding_is_error_execution_and_nothing_is_sent() {
    Harness h;
    h.raise("ghost");
    h.raise("go");
    if (!h.sent.empty()) {
        return fail("a request was sent to a peer the deployment does not declare");
    }
    if (h.brake.errors() != 1u) {
        std::fprintf(stderr, "FAIL: errors is %u after a peer with no binding, want 1\n",
                     static_cast<unsigned>(h.brake.errors()));
        return 1;
    }
    if (h.brake.getCurrentState() != BrakeState::Idle) {
        return fail("the machine did not return to `idle` on error.execution");
    }
    return 0;
}

// A name that is not a `#<machine_name>` is refused before the router is asked.
int a_peer_of_the_wrong_shape_is_error_execution_and_nothing_is_sent() {
    Harness h;
    h.raise("shapeless");
    h.raise("go");
    if (!h.sent.empty()) {
        return fail("a request was sent to a peer that is not a `#<machine_name>`");
    }
    if (h.brake.errors() != 1u) {
        std::fprintf(stderr, "FAIL: errors is %u after a peer of the wrong shape, want 1\n",
                     static_cast<unsigned>(h.brake.errors()));
        return 1;
    }
    return 0;
}

// `overflow` starts a request whose peer is `'#motor_static_srcexpr' + (load * 2000000000)`,
// which a 32-bit field cannot hold: the attribute cannot be evaluated, so it is
// `error.execution` once and nothing is sent, not a request to the half of a name that
// could be read.
int a_peer_that_cannot_be_computed_is_error_execution_and_nothing_is_sent() {
    Harness h;
    h.raise("overflow");
    if (!h.sent.empty()) {
        return fail("a request was sent although its peer could not be computed");
    }
    if (h.brake.errors() != 1u) {
        std::fprintf(stderr, "FAIL: errors is %u after a peer that cannot be computed, want 1\n",
                     static_cast<unsigned>(h.brake.errors()));
        return 1;
    }
    if (h.brake.getCurrentState() != BrakeState::Idle) {
        return fail("the machine did not return to `idle` on error.execution");
    }
    return 0;
}

// The peer is read at each start of the invocation and not once at start-up: a failed
// attempt does not poison the next, and once the field names the binding again the
// request goes.
int the_peer_is_read_again_at_each_invocation() {
    Harness h;
    h.raise("ghost");
    h.raise("go");
    h.raise("home");
    h.raise("go");
    if (h.sent.size() != 1) {
        return fail("the second attempt, with the peer restored, sent one request");
    }
    if (h.brake.errors() != 1u) {
        std::fprintf(stderr, "FAIL: errors is %u after one failed and one good attempt, want 1\n",
                     static_cast<unsigned>(h.brake.errors()));
        return 1;
    }
    return 0;
}

// The reply the peer would send, correlated by the invoke id the request carried: the
// static machine sees `done.invoke` and reaches `ok`, so the request is the one the
// router registered under the name the field held and not one it only sent.
int a_reply_to_the_request_reaches_the_machine_as_done_invoke() {
    Harness h;
    h.raise("go");
    if (h.sent.size() != 1 || !h.sent[0].invoke_id.has_value()) {
        return fail("the request carries the id the reply is correlated by");
    }
    SCE::Mesh::MeshEnvelope reply;
    reply.id = SCE::uuid::v7();
    reply.source = "motor_static_srcexpr";
    reply.type = "service.response.compute_force";
    reply.pattern = SCE::Mesh::PatternKind::RpcReply;
    reply.invoke_id = h.sent[0].invoke_id;
    if (!h.router.dispatchToSession(reply, 0)) {
        return fail("the router accepted the reply");
    }
    h.brake.step();
    if (h.brake.terminalState() != BrakeState::Ok) {
        return fail("the reply did not reach `ok`: done.invoke was not raised");
    }
    return 0;
}

}  // namespace

int main() {
    int bad = 0;
    bad |= a_request_goes_to_the_binding_the_peer_field_names();
    bad |= a_peer_that_matches_no_binding_is_error_execution_and_nothing_is_sent();
    bad |= a_peer_of_the_wrong_shape_is_error_execution_and_nothing_is_sent();
    bad |= a_peer_that_cannot_be_computed_is_error_execution_and_nothing_is_sent();
    bad |= the_peer_is_read_again_at_each_invocation();
    bad |= a_reply_to_the_request_reaches_the_machine_as_done_invoke();
    if (bad != 0) {
        return 1;
    }
    std::printf("mesh static invoke srcexpr: ok\n");
    return 0;
}
