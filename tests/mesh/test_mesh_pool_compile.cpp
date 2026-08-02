// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE-VERIFIES: mesh-14.4
//
// SCE Mesh §14.4 pool compile verification.
//
// Proves that both pool branches of tools/codegen/templates/mesh/cpp/
// mesh_transport.h.jinja2 render compilable C++ against real vsomeip 3.x
// and zenoh-cpp headers:
//
//   * WO7.A — Zenoh pool: runtime KeyExpr concat inside invokeMeshRpc for
//     bindings whose `key:` carries one or more `{name}` placeholders.
//     A pool invoke bypasses `send_zenoh` and dispatches directly via
//     `zenoh_session_->get` with the substituted address.
//
//   * WO7.B — SOME/IP pool: runtime instance-id selection inside
//     invokeMeshRpc for bindings that declare `instance_from:` +
//     `instances:`. init() emits one `request_service(SERVICE, i)` per
//     declared instance (vsomeip's `ANY_INSTANCE` is not a wildcard for
//     request_service — see next_session_phase5_binding_placeholder.md
//     for the analysis) and register_message_handler uses
//     `vsomeip::ANY_INSTANCE` so replies from any instance in the set
//     reach the same correlation table.
//
// Compilation IS the test: `Router router(pool_client);` instantiates
// the class and forces both pool blocks to compile, and taking method
// pointers on `invokeMeshRpc` / `cancelMeshRpc` forces overload
// resolution so a regression that silently drops the pool branch from
// the template surfaces here as "no member named invokeMeshRpc" rather
// than sliding through at an E2E test.

#include "pool_client_sm.h"
#include "pool_client_transport.h"

#include <cstdio>

int main() {
    SCE::Generated::pool_client::pool_client pool_client;
    using Router = SCE::Generated::pool_client::TransportRouter<decltype(pool_client)>;
    Router router({&pool_client});

    // Force overload resolution on the two mesh-rpc entry points so a
    // regression that drops the pool branch from the Jinja2 template
    // surfaces as a compile error at this TU.
    constexpr auto invoke_fn = &Router::invokeMeshRpc;
    constexpr auto cancel_fn = &Router::cancelMeshRpc;
    (void)invoke_fn;
    (void)cancel_fn;

    // SCE_MESH.md §14.4 SOME/IP pool: every instance in the declared
    // `instances:` set is resolved from the vsomeip config alongside the
    // binding-level default. SOMEIP_INSTANCE_CONTROLLER (from
    // vsomeip_pool_controller.json) plus the pool members 0x0002 and
    // 0x0003 are the full set request_service'd at init(). Asserting the
    // default proves the binding-level resolution path is unchanged; the
    // pool extension is proved by the init() loop compiling against
    // `pool_instances_controller` (see generated transport header).
    static_assert(SCE::Generated::pool_client::SOMEIP_SERVICE_CONTROLLER == 0x4567,
                  "service_id must match vsomeip_pool_controller.json");
    static_assert(SCE::Generated::pool_client::SOMEIP_INSTANCE_CONTROLLER == 0x0001,
                  "binding default instance_id must match vsomeip_pool_controller.json");
    static_assert(SCE::Generated::pool_client::SOMEIP_METHOD_CONTROLLER_SERVICE_REQUEST_PING == 0x0101,
                  "method_id must match vsomeip_pool_controller.json methods.ping");

    std::printf("SCE Mesh §14.4 pool compile verification: PASS\n");
    return 0;
}
