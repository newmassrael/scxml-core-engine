// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// docs/adr/0038 under generated C++: a machine that states a duty takes a host that says
// what it does with duties, a machine that states none takes a host with no such member,
// and the opt-in for a simulated host is a type that is written out and not a flag that
// can be passed by accident. The scenario of `static_host_duty` is replayed on a host that
// only records, through that opt-in (static_scenarios/hosts/static_host_duty.h); this file
// holds the machine to the host that performs the duties and to the refusal of one that
// only simulates them.

#include <deque>
#include <stdexcept>
#include <string>
#include <type_traits>
#include <vector>

#include <gtest/gtest.h>

#include "common/SceClock.h"
#include "integration/static_scenarios/StaticScenarioReplay.h"
#include "integration/static_scenarios/hosts/static_host_duty.h"
#include "static_host_call_sm.h"
#include "static_host_duty_child_sm.h"
#include "static_host_duty_sm.h"

namespace SCE::Tests {
namespace {

namespace G = ::SCE::Generated;
using ::SCE::Tests::StaticScenario::DutyHost;
using ::SCE::Tests::StaticScenario::json;

/// Whether `Actions` declares what its host does with duties.
template <typename Actions, typename = void> struct DeclaresRealization : std::false_type {};

template <typename Actions>
struct DeclaresRealization<Actions, std::void_t<decltype(std::declval<const Actions &>().realization())>>
    : std::true_type {};

TEST(AStaticHostDutyTest, OnlyAMachineThatStatesADutyAsksItsHostWhatItDoesWithDuties) {
    static_assert(DeclaresRealization<G::static_host_duty::StaticHostDutyActions>::value,
                  "a machine that states a duty declares the realization in its host interface");
    static_assert(!DeclaresRealization<G::static_host_call::StaticHostCallActions>::value,
                  "a machine of notices takes a host with no realization member");
    static_assert(std::is_constructible_v<G::static_host_duty::static_host_duty,
                                          G::static_host_duty::StaticHostDutyActions &, ::SCE::SimulatedHost>,
                  "the opt-in is the trailing constructor argument of a machine that states a duty");
    static_assert(!std::is_constructible_v<G::static_host_duty::static_host_duty,
                                           G::static_host_duty::StaticHostDutyActions &, bool>,
                  "a bool does not reach the opt-in: it has to be written as SimulatedHost::Accept");
    static_assert(!std::is_constructible_v<G::static_host_call::static_host_call,
                                           G::static_host_call::StaticHostCallActions &, ::SCE::SimulatedHost>,
                  "a machine of notices has no opt-in to give");
}

// A host that says it performs the duties is the host a machine that states one is built
// for: no opt-in is given, and the machine takes the duty on entry and gives it back on
// exit.
TEST(AStaticHostDutyTest, AHostThatPerformsTheDutiesIsAcceptedWithoutAnOptIn) {
    using Machine = G::static_host_duty::static_host_duty;
    DutyHost host(::SCE::ActionRealization::Performs);

    Machine machine(host);
    machine.initialize();
    EXPECT_EQ(host.take(), json::array()) << "`idle` calls nothing on the host";

    machine.raiseExternal("engage");
    machine.step();
    EXPECT_EQ(host.take(), json::parse(R"([{"action": "start_keepalive", "args": []},
                                           {"action": "note_engaged", "args": []}])"))
        << "entering `engaged` takes the duty and then gives the notice";

    machine.raiseExternal("release");
    machine.step();
    EXPECT_EQ(host.take(), json::parse(R"([{"action": "stop_keepalive", "args": []}])"))
        << "leaving `engaged` gives the duty back through its <onexit>";
}

// A host that says it only simulates the duties is refused when the machine is built,
// before it is stored and before the first entry could call it.
TEST(AStaticHostDutyTest, AHostThatOnlySimulatesTheDutiesIsRefusedAtConstruction) {
    using Machine = G::static_host_duty::static_host_duty;
    DutyHost host(::SCE::ActionRealization::Simulates);

    EXPECT_THROW({ Machine machine(host); }, std::invalid_argument);

    std::string refusal;
    try {
        Machine machine(host);
    } catch (const std::invalid_argument &refused) {
        refusal = refused.what();
    }
    EXPECT_NE(refusal.find("static_host_duty"), std::string::npos) << "the message names the machine: " << refusal;
    EXPECT_NE(refusal.find("only simulates"), std::string::npos)
        << "the message says the host declares that it only simulates the duties: " << refusal;
    EXPECT_NE(refusal.find("SimulatedHost::Accept"), std::string::npos) << "the message names the opt-in: " << refusal;
    EXPECT_EQ(host.take(), json::array()) << "a refused host was not called";
}

// The opt-in covers the child the parent builds: the parent's host answers the host of a
// child that states a duty, and the child is built with the opt-in the caller gave the
// parent. Without it the child refuses its host at its own construction.
TEST(AStaticHostDutyTest, TheOptInOfAParentCoversTheChildItBuildsWithItsHost) {
    namespace Pc = G::static_host_duty_child;
    namespace Wk = G::static_host_duty_child__sce_synth_invoke__worker;
    using Machine = Pc::static_host_duty_child;

    struct WorkerHost : Wk::StaticHostDutyChildSceSynthInvokeWorkerActions {
        explicit WorkerHost(::SCE::ActionRealization declared) : declared(declared) {}

        ::SCE::ActionRealization realization() const override {
            return declared;
        }

        void beginWork() override {
            calls.push_back("begin_work");
        }

        void endWork() override {
            calls.push_back("end_work");
        }

        ::SCE::ActionRealization declared;
        std::vector<std::string> calls;
    };

    struct Host : Pc::StaticHostDutyChildActions {
        explicit Host(::SCE::ActionRealization workers) : workers(workers) {}

        Wk::StaticHostDutyChildSceSynthInvokeWorkerActions &actionsForWorker() override {
            return started.emplace_back(workers);
        }

        ::SCE::ActionRealization workers;
        std::deque<WorkerHost> started;
    };

    const auto start = [](Machine &machine) {
        machine.setClock(std::make_shared<SCE::ManualClock>(0));
        machine.initialize();
        ::SCE::Tests::StaticScenario::settleRounds(machine);
    };

    // The parent states no duty, so its own host has no realization to declare.
    static_assert(!DeclaresRealization<Pc::StaticHostDutyChildActions>::value);
    static_assert(DeclaresRealization<Wk::StaticHostDutyChildSceSynthInvokeWorkerActions>::value);

    {
        Host host(::SCE::ActionRealization::Performs);
        Machine machine(host);
        start(machine);
        ASSERT_EQ(host.started.size(), 1u) << "the parent's host was asked for the child's host once";
        EXPECT_EQ(host.started.front().calls, (std::vector<std::string>{"begin_work"}))
            << "the child took its duty on entry from a host that performs it, with no opt-in given";
    }
    {
        Host host(::SCE::ActionRealization::Simulates);
        Machine machine(host);
        // The child refuses a host that only simulates its duty, and the parent's caller
        // gave no opt-in. The refusal is the child's constructor throwing
        // `std::invalid_argument`, and the runtime's pending-invoke loop
        // (`InvokeHelper::executePendingInvokes`) catches a failed start, logs it and goes
        // on with the macrostep, so it does not reach the caller: what the test can hold
        // is that the child never performed a duty on the host that only simulates.
        EXPECT_NO_THROW(start(machine));
        ASSERT_EQ(host.started.size(), 1u) << "the parent's host was asked for the child's host";
        EXPECT_TRUE(host.started.front().calls.empty()) << "the refused host was not called";
    }
    {
        Host host(::SCE::ActionRealization::Simulates);
        Machine machine(host, ::SCE::SimulatedHost::Accept);
        start(machine);
        ASSERT_EQ(host.started.size(), 1u);
        EXPECT_EQ(host.started.front().calls, (std::vector<std::string>{"begin_work"}))
            << "the opt-in the caller gave the parent covers the child the parent builds";
    }
}

}  // namespace
}  // namespace SCE::Tests
