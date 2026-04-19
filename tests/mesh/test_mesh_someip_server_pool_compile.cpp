// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh §14.4 server-side multi-instance pool compile verification.
//
// The motor_pool fixture declares `server.instances: [1, 2]` in
// deploy.yaml. This TU is the static gate that the generated transport
// header renders the pool shape end-to-end:
//
//   * Parse accept: `validate_server_pool_rejection` lets the pool
//     through on SOME/IP (transport registry flag
//     `supports_multi_instance_server = true`).
//   * Topology carry: `ServerBinding.instance_pool` propagates the
//     list to codegen verbatim.
//   * Template emit: `SOMEIP_SERVER_INSTANCES` is a 2-element
//     `std::array<vsomeip::instance_t, N>`; init() iterates it to
//     offer each instance and register per-(instance, method) message
//     handlers.
//   * Session pool: TransportRouter::N_SESSIONS matches the instance
//     count so every offered instance has its own SCXML document
//     session backing it. Inbound SOME/IP handlers resolve
//     `msg->get_instance()` to a `sessions_` slot via
//     `session_index_for_instance()` and dispatch there.
//
// Compilation IS the test: the `static_assert` block exercises the
// array's size, N_SESSIONS, and instance-to-session lookup, and
// instantiating the `Router` with a 2-element session array drags
// the full init() loop + per-session callback install through the C++
// front end. A regression that drops any of these would surface here
// as either an array-size mismatch, an N_SESSIONS mismatch, or a
// missing `SOMEIP_SERVER_INSTANCES` symbol — not as a runtime surprise
// at deploy time.

#include "motor_pool_sm.h"
#include "motor_pool_transport.h"

#include <array>
#include <cstdio>

int main() {
    SCE::Generated::motor_pool::motor_pool motor_a;
    SCE::Generated::motor_pool::motor_pool motor_b;
    using Router = SCE::Generated::motor_pool::TransportRouter<
        SCE::Generated::motor_pool::motor_pool>;

    // Parse accepted `instances: [1, 2]` and codegen emitted a
    // 2-element instance array. A future change that reverts to the
    // singular `SOMEIP_SERVER_INSTANCE` constant would break the
    // `.size()` / `.at()` access below at compile time.
    static_assert(SCE::Generated::motor_pool::SOMEIP_SERVER_INSTANCES.size() == 2,
                  "pool server must emit one array entry per declared instance");
    static_assert(SCE::Generated::motor_pool::SOMEIP_SERVER_INSTANCES[0] == 0x0001,
                  "first pool member must match deploy.yaml instances: [1, 2]");
    static_assert(SCE::Generated::motor_pool::SOMEIP_SERVER_INSTANCES[1] == 0x0002,
                  "second pool member must match deploy.yaml instances: [1, 2]");

    // Session pool cardinality must equal the offered-instance count
    // so every pool member has its own SCXML document session
    // (SCE_MESH.md §14.4). A regression that collapses sessions to 1
    // while keeping two offered instances would be silent at runtime
    // (both instances would route to sessions_[0]) — pin it at compile
    // time.
    static_assert(Router::N_SESSIONS == 2,
                  "server pool N_SESSIONS must match offered-instance count");

    // Service + method ids stay single-valued — only the instance
    // dimension is multi-valued for a pool. vsomeip_motor_pool.json is
    // the source of truth for both.
    static_assert(SCE::Generated::motor_pool::SOMEIP_SERVER_SERVICE == 0x3000,
                  "service_id must match vsomeip_motor_pool.json");
    static_assert(SCE::Generated::motor_pool::SOMEIP_SERVER_METHOD_SERVICE_REQUEST_COMPUTE == 0x0101,
                  "method_id must match vsomeip_motor_pool.json methods.compute");

    // Instance-to-session lookup must fold the declared set into the
    // array positions; an unknown instance must return the
    // `N_SESSIONS` sentinel so the inbound handler's drift guard
    // catches vsomeip dispatching for an un-offered instance.
    static_assert(Router::session_index_for_instance(0x0001) == 0,
                  "instance 0x0001 must map to sessions_[0]");
    static_assert(Router::session_index_for_instance(0x0002) == 1,
                  "instance 0x0002 must map to sessions_[1]");
    static_assert(Router::session_index_for_instance(0x00FF) == Router::N_SESSIONS,
                  "unknown instance must return N_SESSIONS sentinel");

    // Instantiating the router with a 2-element session array forces
    // the init() loop through overload resolution: the per-instance
    // `offer_service` / `register_message_handler` / `offer_event`
    // calls, the per-session `setMeshSendCallback` loop, and the
    // `sessions_` ctor initializer all instantiate here. A template
    // regression that drops any of these surfaces as a compile error.
    Router router({&motor_a, &motor_b});
    (void)router;

    std::printf("SCE Mesh §14.4 server pool compile verification: PASS\n");
    return 0;
}
