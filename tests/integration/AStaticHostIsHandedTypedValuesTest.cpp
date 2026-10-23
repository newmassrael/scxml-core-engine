// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// datamodel="sce-static" (docs/SCE_ACCEPTED_SUBSET.md §2.15) under generated C++: what a
// host is handed. A host action takes the machine's variables as typed arguments, each
// read when the call is made, and an argument that cannot be computed stops the call; a
// published byte string is lent, not copied.

#include <cstdint>
#include <string>
#include <type_traits>
#include <utility>
#include <vector>

#include <gtest/gtest.h>

#include "static_bytes_sm.h"
#include "static_host_call_arguments_sm.h"
#include "static_host_call_sm.h"

namespace SCE::Tests {
namespace {

namespace G = ::SCE::Generated;

/// What a host recorded of the calls a machine made, as `(name, value…)` text.
using Calls = std::vector<std::string>;

// A host action takes the machine's variables as typed arguments, each read when the call
// is made: one call per entry of `idle`, with the datamodel as it stood.
TEST(AStaticHostIsHandedTypedValuesTest, AHostActionTakesTypedDatamodelArguments) {
    namespace Hc = G::static_host_call;

    struct Host : Hc::StaticHostCallActions {
        Calls calls;

        void showAttempts(uint32_t count, bool exhausted) override {
            calls.push_back(std::to_string(count) + (exhausted ? ",true" : ",false"));
        }
    } host;

    Hc::static_host_call machine(host);
    machine.initialize();
    for (int i = 0; i < 4; ++i) {
        machine.raiseExternal("retry");
        machine.step();
    }
    // The fourth retry finds `attempts < 3` false and re-enters nothing.
    EXPECT_EQ(host.calls, (Calls{"0,false", "1,false", "2,false", "3,true"}));
}

// An argument that cannot be computed is a failure, not a wrapped value: the host is not
// called, and `error.execution` is raised in the call's place.
TEST(AStaticHostIsHandedTypedValuesTest, AnArgumentThatOverflowsStopsTheCallAndRaisesAnError) {
    namespace Hc = G::static_host_call_arguments;

    struct Host : Hc::StaticHostCallArgumentsActions {
        Calls calls;

        void report(uint8_t next) override {
            calls.push_back(std::to_string(next));
        }
    } host;

    Hc::static_host_call_arguments machine(host);
    machine.initialize();

    machine.raiseExternal("fine");
    machine.step();
    EXPECT_EQ(host.calls, (Calls{"251"})) << "250 + 1 fits a uint8";
    EXPECT_EQ(machine.errors(), 0);

    machine.raiseExternal("overflow");
    machine.step();
    EXPECT_EQ(host.calls, (Calls{"251"})) << "250 + 10 does not fit, so the host is not called with 4";
    EXPECT_EQ(machine.errors(), 1) << "error.execution was raised and the machine saw it";
}

// A host is handed a byte string as a constant reference: the vector is the machine's own,
// and a write into it would change the variable behind its bound. Checked where it is
// compiled, since a reader that returned a copy or a mutable reference would be another
// type.
static_assert(std::is_same_v<decltype(std::declval<const G::static_bytes::static_bytes &>().frame()),
                             const std::vector<std::uint8_t> &>,
              "a published byte string is lent as a constant reference");

}  // namespace
}  // namespace SCE::Tests
