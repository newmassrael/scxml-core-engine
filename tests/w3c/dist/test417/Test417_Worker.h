// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh §16.8 seed — NonRoot partition registration for W3C test417.
//
// Hosts the `s1p12` region. `initialize()` drives the local region
// through its NULL-transition chain into `<final id="s1p12final">`;
// the NonRoot branch of `mesh/cpp/parallel_final.jinja2` synchronously
// calls `sm.sendParallelRegionDone("s1p1", "s1p12", …)`, which routes
// through the `TransportRouter` ctor's outbound callback into
// `wire21_to_test417_main_.send(env)`. Returns 0 after a brief hold
// so the Root partition has time to drain the shm segment before the
// router dtor unlinks it.

#pragma once

#include "PartitionRunner.h"
#include "test417_sm.h"
#include "test417_transport.h"

#include <chrono>
#include <cstdio>
#include <string>
#include <thread>

namespace SCE::W3C::Dist {

struct Test417_Worker : public PartitionBase {
    static constexpr const char *TEST_ID = "417";
    static constexpr const char *PARTITION = "test417_worker";

    int run(const std::string & /*deploy_dir*/) override {
        namespace gen = SCE::Generated::test417::P_test417_worker;

        gen::test417 sm;
        // Router ctor creates the outbound wire-21 shm channel in
        // `Mode::Create` and installs the SM's
        // `setParallelRegionDoneCallback` closure that routes to
        // `wire21_to_test417_main_.send(env)`.
        gen::TransportRouter<gen::test417> router({&sm});

        // `initialize()` walks `s1` → `s1p1` → (`s1p11`, `s1p12`)
        // and fires the NULL transition `s1p121` → `s1p12final`.
        // On entry to `s1p12final`, the NonRoot codegen branch calls
        // `sm.sendParallelRegionDone("s1p1", "s1p12", donedata)`, which
        // dispatches through the installed callback to the wire-21
        // shm segment.
        sm.initialize();

        // Hold the shm segment open briefly so the Root reader can
        // drain before `router` goes out of scope and unlinks. Mirrors
        // the 2 s grace period in the rule-12 NonRoot proc
        // (`tests/mesh/test_mesh_partition_rule12_right_proc.cpp`);
        // the test-local 1 s timeout on `<send event="timeout">` in
        // the Root would fire first if the Root is stalled, so the
        // wait here is a one-shot handshake grace, not a polling loop.
        std::this_thread::sleep_for(std::chrono::milliseconds(1500));

        std::fprintf(stdout,
                     "test417 worker: PASS — wire-21 ParallelRegionDone dispatched for region "
                     "\"s1p12\"\n");
        return 0;
    }
};

inline static PartitionRegistrar<Test417_Worker> registrar_Test417_Worker;

}  // namespace SCE::W3C::Dist
