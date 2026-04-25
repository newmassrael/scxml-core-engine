// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh RFC F.X-2 D8 — defense-in-depth for the vsomeip → SCE
// callback boundary.
//
// `SCE::Mesh::Someip::invokeReceiveSafely` is the single boundary every
// §9.6 SOMEIP wire-14..20 envelope crosses from vsomeip's callback thread
// into SCE handler code under the consolidated `<machine>[_<partition>]_sce`
// app. Under per-subsystem split (pre-F.X-2), an exception that escaped
// a §9.6 callback would only block §9.6 traffic — the dedicated app's
// callback thread. Under consolidation, that same exception would block
// every SCE-reserved subsystem on this binary. The boundary's job is to
// catch every exception (typed and untyped) and let vsomeip continue
// dispatching subsequent envelopes unaffected.
//
// Tests below pin three properties:
//   1. A `std::exception` thrown from the SCE handler does not propagate.
//   2. A non-`std::exception` (anything reaching the `catch (...)` arm)
//      also does not propagate.
//   3. After an exception is swallowed, a subsequent invocation with a
//      well-behaved handler still runs to completion — the boundary does
//      not leave the runtime in a wedged state.
//
// `invokeReceiveSafely` is `noexcept`, so the compiler enforces "no
// escape" at the type level — a regression that removes the catch-all
// arm trips at compile time, not runtime. These tests still exist to
// pin the *observable* behaviour (handler invoked, exception swallowed,
// subsequent dispatch works) so a future refactor that replaces the
// implementation cannot regress observable semantics undetected.

#include "mesh/transports/SomeipScxmlInvokeEndpoint.h"

#include <atomic>
#include <cstdio>
#include <stdexcept>
#include <string>

namespace {

using SCE::Mesh::Someip::invokeReceiveSafely;
using SCE::Mesh::Someip::ReceiveCallback;

// Construct an envelope whose only purpose is to be passed through the
// boundary unmodified. Field values are arbitrary — the tests assert on
// callback observation, not on envelope content.
SCE::Mesh::MeshEnvelope make_probe_envelope() {
    SCE::Mesh::MeshEnvelope env;
    env.pattern = SCE::Mesh::PatternKind::InvokeStarted;
    env.source = "probe_source";
    env.type = "probe.event";
    return env;
}

#define REQUIRE(cond, msg)                                              \
    do {                                                                \
        if (!(cond)) {                                                  \
            std::fprintf(stderr, "FAIL: %s (%s:%d)\n", msg, __FILE__, __LINE__); \
            return 1;                                                   \
        }                                                               \
    } while (0)

int test_std_exception_does_not_propagate() {
    std::atomic<int> calls{0};
    ReceiveCallback throws_runtime = [&](const SCE::Mesh::MeshEnvelope&) {
        calls.fetch_add(1);
        throw std::runtime_error("simulated SCE handler failure");
    };

    auto env = make_probe_envelope();
    invokeReceiveSafely(throws_runtime, env);

    REQUIRE(calls.load() == 1, "handler must have been invoked exactly once");
    return 0;
}

int test_non_std_exception_does_not_propagate() {
    std::atomic<int> calls{0};
    ReceiveCallback throws_int = [&](const SCE::Mesh::MeshEnvelope&) {
        calls.fetch_add(1);
        throw 42;  // non-std::exception → must reach `catch (...)` arm
    };

    auto env = make_probe_envelope();
    invokeReceiveSafely(throws_int, env);

    REQUIRE(calls.load() == 1,
            "handler must have been invoked exactly once "
            "(non-std::exception still reaches the body)");
    return 0;
}

int test_null_callback_is_safe() {
    // Empty `std::function` → early return, never crashes the boundary.
    ReceiveCallback empty;
    auto env = make_probe_envelope();
    invokeReceiveSafely(empty, env);
    return 0;
}

int test_subsequent_dispatch_after_exception_still_runs() {
    // After the boundary swallows an exception, the next invocation with
    // a well-behaved handler must still execute. The boundary must not
    // leave any persistent state behind.
    std::atomic<int> good_calls{0};
    ReceiveCallback throws_once = [](const SCE::Mesh::MeshEnvelope&) {
        throw std::logic_error("first handler throws");
    };
    ReceiveCallback well_behaved = [&](const SCE::Mesh::MeshEnvelope&) {
        good_calls.fetch_add(1);
    };

    auto env = make_probe_envelope();
    invokeReceiveSafely(throws_once, env);   // exception swallowed
    invokeReceiveSafely(well_behaved, env);  // must run to completion

    REQUIRE(good_calls.load() == 1,
            "well-behaved handler must run after an earlier exception was "
            "swallowed by the boundary");
    return 0;
}

int test_repeated_throwing_handler_runs_each_time() {
    // The boundary is reentrant on its own thread — repeated invocations
    // of a throwing handler each get caught independently. Pins that the
    // catch-all does not stash exception state across calls.
    std::atomic<int> calls{0};
    ReceiveCallback throws_each_time = [&](const SCE::Mesh::MeshEnvelope&) {
        calls.fetch_add(1);
        throw std::runtime_error("recurring failure");
    };

    auto env = make_probe_envelope();
    for (int i = 0; i < 5; ++i) {
        invokeReceiveSafely(throws_each_time, env);
    }

    REQUIRE(calls.load() == 5,
            "each throwing call must be invoked AND swallowed independently");
    return 0;
}

}  // namespace

int main() {
    int rc = 0;
    rc |= test_std_exception_does_not_propagate();
    rc |= test_non_std_exception_does_not_propagate();
    rc |= test_null_callback_is_safe();
    rc |= test_subsequent_dispatch_after_exception_still_runs();
    rc |= test_repeated_throwing_handler_runs_each_time();
    if (rc == 0) {
        std::printf("SCE Mesh RFC F.X-2 D8 invokeReceiveSafely boundary: PASS\n");
    }
    return rc;
}
