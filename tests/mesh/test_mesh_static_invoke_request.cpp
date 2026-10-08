// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE_MESH.md §9.5 mesh-rpc under `datamodel="sce-static"` (docs/adr/0005, decision 5):
// the `<param>`s of a Mesh request are read from the machine's own fields when the
// invocation starts, and cross as the request's payload, by the router the build
// generates for the deployment.
//
// Fixture: tests/mesh/brake_static_invoke.scxml, with `motor_static_invoke.scxml` as the
// peer the router is parameterised on and `deploy_static_invoke.yaml` as the
// deployment. The request is observed where the router hands it to its link, so no
// transport is involved and the peer never runs; the reply the last case needs is built
// by hand, as `test_mesh_cross_target_rpc_reply.cpp` builds one.
//
// The machine has no script engine, and the test is linked without one: the link is the
// proof that no param reached an engine.

#include "brake_static_invoke_sm.h"
#include "brake_static_invoke_transport.h"
#include "motor_static_invoke_sm.h"

#include "mesh/MeshEnvelope.h"

#include <algorithm>
#include <cstdio>
#include <optional>
#include <string>
#include <vector>

namespace {

using Brake = SCE::Generated::brake_static_invoke::brake_static_invoke;
using BrakeState = SCE::Generated::brake_static_invoke::State;
using Motor = SCE::Generated::motor_static_invoke::motor_static_invoke;
using Router = SCE::Generated::brake_static_invoke::TransportRouter<Brake, Motor>;

/// A machine and its router, with the requests the router handed to its link.
struct Harness {
    Brake brake;
    Motor motor;  // the peer the router's template is parameterised on; the link below
                  // swallows the request, so it never runs.
    Router router;
    std::vector<SCE::Mesh::MeshEnvelope> sent;

    Harness() : router({&brake}, motor) {
        router.linkTo("#motor_static_invoke", [this](const SCE::Mesh::MeshEnvelope &env) {
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

bool holds(const std::string &body, const char *pair) {
    return body.find(pair) != std::string::npos;
}

// `bump` makes the fields 4 and true before `go` starts the invocation, so what the
// request carries is what they hold now and not what they held at start-up.
int a_request_carries_the_values_the_fields_hold_when_it_starts() {
    Harness h;
    h.raise("bump");
    h.raise("go");
    if (h.sent.size() != 1) {
        return fail("one request is handed to the router's link");
    }
    const auto &env = h.sent[0];
    if (env.type != "service.request.compute_force") {
        std::fprintf(stderr, "FAIL: the envelope's type is `%s`, not the `_mesh_event` constant\n", env.type.c_str());
        return 1;
    }
    const std::string body = payloadOf(env);
    for (const char *pair : {"\"load\":4", "\"engaged\":true", "\"doubled\":8", "\"gain\":1.5"}) {
        if (!holds(body, pair)) {
            std::fprintf(stderr, "FAIL: the payload `%s` does not hold `%s`\n", body.c_str(), pair);
            return 1;
        }
    }
    if (holds(body, "\"_mesh_event\"")) {
        return fail("the event name is a constant of the envelope and never a pair of the payload");
    }
    return 0;
}

// The control: nothing has written a field, so the request carries what `<data expr>`
// gave them, not the values `bump` would have set.
int the_pairs_are_the_fields_as_they_stand_and_not_a_copy_from_start_up() {
    Harness h;
    h.raise("go");
    if (h.sent.size() != 1) {
        return fail("one request is handed to the router's link");
    }
    const std::string body = payloadOf(h.sent[0]);
    for (const char *pair : {"\"load\":3", "\"engaged\":false", "\"doubled\":6", "\"gain\":1.5"}) {
        if (!holds(body, pair)) {
            std::fprintf(stderr, "FAIL: before any bump the payload `%s` does not hold `%s`\n", body.c_str(), pair);
            return 1;
        }
    }
    return 0;
}

// `boom` is `load * 2000000000`, which a 32-bit field cannot hold: its pair is left out,
// not carried as a zero, the request still goes with the pairs that could be read, and
// the failure is counted as error.execution once (§scxml-5.7.1).
int a_param_that_cannot_be_read_is_left_out_and_the_request_still_goes() {
    Harness h;
    h.raise("bump");
    h.raise("go");
    if (h.sent.size() != 1) {
        return fail("the request goes with the pairs that could be read");
    }
    if (holds(payloadOf(h.sent[0]), "\"boom\"")) {
        return fail("the pair whose value failed was carried");
    }
    if (h.brake.errors() != 1u) {
        std::fprintf(stderr, "FAIL: errors is %u, want 1\n", static_cast<unsigned>(h.brake.errors()));
        return 1;
    }
    return 0;
}

// The reply the peer would send, correlated by the invoke id the request carried:
// the static machine sees `done.invoke` and reaches its `ok` state, so the request is
// the one the router registered and not one it only sent.
int a_reply_to_the_request_reaches_the_machine_as_done_invoke() {
    Harness h;
    h.raise("go");
    if (h.sent.size() != 1 || !h.sent[0].invoke_id.has_value()) {
        return fail("the request carries the id the reply is correlated by");
    }
    if (h.brake.getCurrentState() != BrakeState::Computing) {
        return fail("the machine waits in `computing` for the reply");
    }
    SCE::Mesh::MeshEnvelope reply;
    reply.id = SCE::uuid::v7();
    reply.source = "motor_static_invoke";
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
    bad |= a_request_carries_the_values_the_fields_hold_when_it_starts();
    bad |= the_pairs_are_the_fields_as_they_stand_and_not_a_copy_from_start_up();
    bad |= a_param_that_cannot_be_read_is_left_out_and_the_request_still_goes();
    bad |= a_reply_to_the_request_reaches_the_machine_as_done_invoke();
    if (bad != 0) {
        return 1;
    }
    std::printf("mesh static invoke request: ok\n");
    return 0;
}
