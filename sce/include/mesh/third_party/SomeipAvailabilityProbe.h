// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh — axis-6 third-party library surface absorber (vsomeip
// initial-edge callback claim). docs/SCE_AXIS_6_PATTERNS.md A6-002.
//
// vsomeip's documented contract for `register_availability_handler` is
// "the callback is invoked on every state change including the initial
// NOT_AVAILABLE → AVAILABLE transition" — SCE's codegen TransportRouter
// init relies on that initial-edge callback to drive the per-target
// `OutboundBuffer::markReady()`. If vsomeip ever ships an
// "initial-callback-debounce" optimization, a first-callback semantics
// change in any minor release, or a future API that batches state
// changes, the buffer would never drain and SCE would silently hang at
// startup with no error event.
//
// `probeAndDispatch` absorbs that risk with a defensive idempotent
// pattern: register the handler, then synthesize a single immediate
// invocation passing the current `is_available()` state. The caller's
// handler runs at least once regardless of vsomeip's callback policy.
// If vsomeip DOES fire the initial callback, the handler runs twice
// with the same state and the OutboundBuffer's `markReady`/`markNotReady`
// idempotency absorbs the duplicate — `markReady()` from an already-
// ready buffer is a no-op, and the codegen-emitted row-10 one-shot
// `<target>_auth_unauthorized_fired_` is gated by `exchange(true)` so
// the second call cannot double-raise.
//
// Single-call API by design — a two-call `registerHandler()` +
// `probeNow()` API would let a caller skip the probe step and lose the
// absorption guarantee. The CI fixture
// (`tests/mesh/SomeipAvailabilityProbeTest.cpp`) pins the contract
// against a mock vsomeip application that does NOT fire the initial
// callback, asserting the handler is still invoked exactly once via
// the probe path with the correct availability state.

#pragma once

#include <memory>

// `vsomeip.hpp` is the umbrella header that establishes the
// `namespace vsomeip = vsomeip_v3;` alias the rest of SCE uses. Lower-
// level headers (`application.hpp`, `handler.hpp`, `primitive_types.hpp`)
// only declare the types under `vsomeip_v3` and would leave
// `vsomeip::availability_handler_t` unresolved when included alone.
#include <vsomeip/vsomeip.hpp>

namespace SCE::Mesh::ThirdParty {

/// Register `handler` for (`service`, `instance`) availability events
/// and synthesize a single immediate invocation carrying the current
/// `is_available()` state. The caller's handler is guaranteed to run
/// at least once even if vsomeip's `register_availability_handler`
/// does not fire the initial callback. Subsequent vsomeip-originated
/// callbacks for the same (`service`, `instance`) pair flow through
/// normally.
///
/// Idempotency requirement on `handler`: the handler must tolerate
/// being invoked twice with the same (`service`, `instance`,
/// `is_available`) tuple. SCE's codegen-emitted handler satisfies this
/// (OutboundBuffer markReady / markNotReady are idempotent; row-10
/// auth one-shot is `exchange(true)` gated). External callers
/// adopting this absorber must verify the same.
///
/// Templated on `App` so the unit test can pass a minimal mock without
/// inheriting the full `vsomeip::application` abstract interface.
/// Production code passes the live `std::shared_ptr<vsomeip::application>`;
/// the template parameter is inferred and adds no ABI surface.
template <typename App>
inline void probeAndDispatch(const std::shared_ptr<App> &app, vsomeip::service_t service, vsomeip::instance_t instance,
                             const vsomeip::availability_handler_t &handler) {
    app->register_availability_handler(service, instance, handler);
    handler(service, instance, app->is_available(service, instance));
}

}  // namespace SCE::Mesh::ThirdParty
