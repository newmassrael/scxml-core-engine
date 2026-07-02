// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh §9.5 srcexpr compile verification.
//
// Until this fixture landed, no SCXML in-tree used
// `<invoke type="sce:mesh-rpc" srcexpr="...">`, so the
// `{% if invoke_info.srcexpr %}` branches of
// `tools/codegen/templates/entry_exit_actions.jinja2` (entry eval +
// regex shape check + stash) and the paired onexit cancel, plus the
// class-scope `_mesh_resolved_src_<suffix>_` member declaration in
// `state_machine.jinja2`, emitted no compiled C++. Every existing
// fixture exercises the literal-`src` code path, so the srcexpr
// site was built-but-unconsumed in the textbook sense.
//
// The fixture forces every srcexpr code site into one translation
// unit:
//   * `calling_literal` — static `src="#motor_srcexpr"` so the
//     topology yields a non-empty `invoke_sites_by_target`, and the
//     generated `TransportRouter::invokeMeshRpc` has a dispatch
//     entry to match against. Without this the router reduces to a
//     bare fall-through `return false;` and the srcexpr branch's
//     per-field_suffix match is never paired with a real site.
//   * `calling_runtime` — `srcexpr="target_name"` reads the
//     datamodel variable and emits the full entry/exit srcexpr block
//     including `ensureScriptEngine()`, `evaluateExpression(...)`,
//     `std::regex_match(...)`, the `_mesh_resolved_src_<suffix>_`
//     stash, and the paired `performMeshCancel(resolved, ...)`.
//
// Compilation IS the test — instantiating the policy's executeEntry/
// ExitActions templates and taking the router's method pointers
// forces template instantiation of every branch without requiring
// the script engine to actually run.

#include "motor_srcexpr_sm.h"
#include "srcexpr_client_sm.h"
#include "srcexpr_client_transport.h"

#include <cstdio>

int main() {
    SCE::Generated::srcexpr_client::srcexpr_client client;
    SCE::Generated::motor_srcexpr::motor_srcexpr motor;

    using Router = SCE::Generated::srcexpr_client::TransportRouter<decltype(client), decltype(motor)>;
    Router router({&client}, motor);

    // Force overload resolution on the router's mesh-rpc method
    // pointers. `invokeMeshRpc` / `cancelMeshRpc` are template-free
    // on the Router, so taking their addresses fully commits them to
    // this TU — a regression that drops either from the template
    // surfaces here as "no member named" rather than sliding through.
    constexpr auto invoke_fn = &Router::invokeMeshRpc;
    constexpr auto cancel_fn = &Router::cancelMeshRpc;
    (void)invoke_fn;
    (void)cancel_fn;

    // Force instantiation of the policy's entry/exit template methods
    // under the concrete engine type. `executeEntryActions<Engine>` is
    // a template method on `srcexpr_clientPolicy`, so merely
    // constructing the derived `srcexpr_client` instance above does
    // not compile its body. Taking a method pointer with explicit
    // template arguments is the minimal invocation that emits the
    // full body, including the srcexpr entry / regex-shape / stash
    // code and the paired exit cancel.
    using Policy = SCE::Generated::srcexpr_client::srcexpr_clientPolicy;
    using SM = SCE::Generated::srcexpr_client::srcexpr_client;
    constexpr auto entry_fn = &Policy::template executeEntryActions<SM>;
    constexpr auto exit_fn = &Policy::template executeExitActions<SM>;
    (void)entry_fn;
    (void)exit_fn;

    std::printf("SCE Mesh §9.5 srcexpr compile verification: PASS\n");
    return 0;
}
