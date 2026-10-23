// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE_MESH.md, Errors, under generated C++: a send to a Mesh peer from a session whose host
// gave it no router is refused where it is made, as `error.execution`, and nothing falls
// back to another router. The scenarios replay these machines with the routers a host
// registers (static_scenarios/StaticScenarioReplay.h); this file holds what happens with
// none.

#include <memory>
#include <string>

#include <gtest/gtest.h>

#include "common/SceClock.h"
#include "integration/static_scenarios/StaticScenarioReplay.h"
#include "static_child_mesh_sm.h"
#include "todo_peer_sm.h"

namespace SCE::Tests {
namespace {

namespace G = ::SCE::Generated;

// A child given no router has its peer send refused as `error.execution` in the child, and
// the machine's own router is not asked: nothing falls back to it.
TEST(AStaticMeshRouterTest, AChildGivenNoMeshRouterHasItsSendRefused) {
    using Machine = G::static_child_mesh::static_child_mesh;
    Machine machine;
    int parentSends = 0;
    machine.setClock(std::make_shared<SCE::ManualClock>(0));
    machine.setMeshSendCallback([&parentSends](const std::string &, const std::string &, const std::string &,
                                               const std::string &, const std::string &) {
        ++parentSends;
        return true;
    });
    machine.initialize();
    ::SCE::Tests::StaticScenario::settleRounds(machine);
    EXPECT_EQ(machine.refusals(), 1u) << "the child ended on its refused greeting, and the machine counted it";
    EXPECT_EQ(parentSends, 0) << "a child's send fell back to the machine's router";
}

// A send to a peer from a machine whose host registered no Mesh router is a configuration
// fault of the host and raises error.execution, as it does on every other backend. It does
// not go to the sender's own queue: todo_peer takes what a peer sends it as a write to its
// own list, so an event that came back would be taken for the peer's, and the one the
// machine sent itself over `#_internal` is the only write it holds.
TEST(AStaticMeshRouterTest, ASendToAMeshPeerWithNoRouterRegisteredIsErrorExecution) {
    G::todo_peer::todo_peer machine;
    machine.initialize();
    machine.raiseExternal("todo.join", R"({"node":"1","hub":false})");
    machine.raiseExternal("todo.edit", R"({"id":7,"field":1,"value":"0","text":"buy milk","now":"0"})");
    machine.step();
    EXPECT_EQ(machine.refusals(), 1u) << "the send to #server was refused, once, as error.execution";
}

}  // namespace
}  // namespace SCE::Tests
