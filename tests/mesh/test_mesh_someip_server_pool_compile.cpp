// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh §14.4 (Gap 7) server-side multi-instance pool compile
// verification.
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
//
// Compilation IS the test: the `static_assert` block exercises the
// array's size and members, and instantiating the `Router` type drags
// the full init() loop through the C++ front end. A regression that
// drops the pool path from the Jinja2 template would surface here as
// either an array-size mismatch or a missing `SOMEIP_SERVER_INSTANCES`
// symbol — not as a runtime surprise at deploy time.
//
// Runtime per-instance dispatch (`msg->get_instance()` → per-session
// SCXMLSession) is deliberately out of this commit's scope; the
// server remains coarse (every inbound admits through the same
// `this->` dispatch path) until a separate Gap 7 step lands the
// instance-axis routing.

#include "motor_pool_sm.h"
#include "motor_pool_transport.h"

#include <array>
#include <cstdio>

int main() {
    SCE::Generated::motor_pool::motor_pool motor;
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

    // Service + method ids stay single-valued — only the instance
    // dimension is multi-valued for a pool. vsomeip_motor_pool.json is
    // the source of truth for both.
    static_assert(SCE::Generated::motor_pool::SOMEIP_SERVER_SERVICE == 0x3000,
                  "service_id must match vsomeip_motor_pool.json");
    static_assert(SCE::Generated::motor_pool::SOMEIP_SERVER_METHOD_SERVICE_REQUEST_COMPUTE == 0x0101,
                  "method_id must match vsomeip_motor_pool.json methods.compute");

    // Instantiating the router forces the init() loop through overload
    // resolution: the per-instance `offer_service` /
    // `register_message_handler` / (here) `offer_event` calls all
    // reference `server_instance` by the `for (auto server_instance :
    // SOMEIP_SERVER_INSTANCES)` header, so any template regression that
    // drops the loop variable surfaces here.
    Router router(motor);
    (void)router;

    std::printf("SCE Mesh §14.4 server pool compile verification: PASS\n");
    return 0;
}
