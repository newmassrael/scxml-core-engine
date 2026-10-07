// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// SCE Accepted Subset §2.15, "Child sessions" (docs/adr/0005, decision 6): a
// child that declares `<sce:action>`s takes the host that performs them when it
// is built, because its first `<onentry>` can already perform an act — a host
// installed afterwards would arrive one act too late. So the host has to exist
// when the invocation starts, and the parent obtains it from its own host, which
// answers one for the child each time the invocation starts.
//
// `static_child_host.scxml` invokes `worker`, which announces itself on entry
// (`started`) and reports its steps when it ends (`finished`). The parent
// declares no act: its host is there for the child alone.
// `static_child_host_hybrid.scxml` invokes whichever of two candidates its
// `srcexpr` names, each with an act of its own, and the host answered is the one
// for THAT candidate. A C++ machine is not saved, so no restore is read here; the
// Kotlin, Python and Go halves are `AChildIsGivenItsHostByItsParentTest.kt`,
// `test_a_child_is_given_its_host_by_its_parent.py` and
// `a_child_is_given_its_host_by_its_parent_test.go`.
//
// The parent's host owns what it answers — a reference, as a machine's own host
// is held — so these hosts keep every one in a deque, whose elements stay put.

#include <gtest/gtest.h>

#include <cstdint>
#include <deque>
#include <string>
#include <vector>

#include "static_child_host_hybrid_sm.h"
#include "static_child_host_sm.h"

namespace G = ::SCE::Generated;

namespace {

using Calls = std::vector<std::string>;

/// The host a `worker` child performs its acts through; it keeps what it was asked.
struct WorkerHost : G::static_child_host__sce_synth_invoke__worker::StaticChildHostSceSynthInvokeWorkerActions {
    Calls calls;

    void started() override {
        calls.push_back("started");
    }

    void finished(uint32_t steps) override {
        calls.push_back("finished(" + std::to_string(steps) + ")");
    }
};

/// The parent's host: it answers a fresh child host for each question and keeps
/// every one, in the order asked.
struct ParentHost : G::static_child_host::StaticChildHostActions {
    int asked = 0;
    std::deque<WorkerHost> workers;

    G::static_child_host__sce_synth_invoke__worker::StaticChildHostSceSynthInvokeWorkerActions &
    actionsForWorker() override {
        ++asked;
        return workers.emplace_back();
    }
};

class AChildIsGivenItsHostByItsParent : public ::testing::Test {
protected:
    void SetUp() override {
        machine = std::make_unique<G::static_child_host::static_child_host>(host);
        machine->initialize();
        settle();
    }

    /// Let the child run: a child that ends during its own start has reported by
    /// the time the parent's start returns, and what it is sent is forwarded.
    void settle() {
        for (int i = 0; i < 40; ++i) {
            machine->tick();
        }
    }

    void send(const std::string &name) {
        machine->raiseExternal(name);
        settle();
    }

    ParentHost host;
    std::unique_ptr<G::static_child_host::static_child_host> machine;
};

}  // namespace

TEST_F(AChildIsGivenItsHostByItsParent, TheChildPerformsItsFirstActThroughTheHostItsParentAnswered) {
    ASSERT_EQ(host.asked, 1) << "the parent's host was asked once";
    // The act of its first `<onentry>` is already performed: the host was there
    // when the child was built, not installed after.
    EXPECT_EQ(host.workers[0].calls, (Calls{"started"}));
}

TEST_F(AChildIsGivenItsHostByItsParent, TheChildReportsWhatItDidThroughTheSameHost) {
    send("a");
    send("b");
    EXPECT_EQ(machine->completed(), 1u) << "the child ended and the parent counted it";
    EXPECT_EQ(host.workers[0].calls, (Calls{"started", "finished(2)"}));
    EXPECT_EQ(host.asked, 1) << "nothing asked the parent's host again";
}

TEST_F(AChildIsGivenItsHostByItsParent, AStateInvokedAgainIsGivenAHostOfItsOwn) {
    for (const char *name : {"a", "b", "again", "back"}) {
        send(name);
    }
    ASSERT_EQ(host.asked, 2) << "asked once per start";
    ASSERT_EQ(host.workers.size(), 2u);
    // The first child's run is its own, and the second starts from nothing.
    EXPECT_EQ(host.workers[0].calls, (Calls{"started", "finished(2)"}));
    EXPECT_EQ(host.workers[1].calls, (Calls{"started"}));
}

namespace {

namespace Hy = G::static_child_host_hybrid;

struct FirstHost : G::static_hosted_first::StaticHostedFirstActions {
    Calls calls;

    void firstRan() override {
        calls.push_back("first_ran");
    }
};

struct SecondHost : G::static_hosted_second::StaticHostedSecondActions {
    Calls calls;

    void secondRan() override {
        calls.push_back("second_ran");
    }
};

/// The hybrid parent's host: it answers the host for the candidate it is asked
/// about, and records which candidate that was.
struct HybridParentHost : Hy::StaticChildHostHybridActions {
    Calls asked;
    std::deque<FirstHost> firsts;
    std::deque<SecondHost> seconds;

    G::static_hosted_first::StaticHostedFirstActions &actionsForWorkStaticHostedFirst() override {
        asked.push_back("first");
        return firsts.emplace_back();
    }

    G::static_hosted_second::StaticHostedSecondActions &actionsForWorkStaticHostedSecond() override {
        asked.push_back("second");
        return seconds.emplace_back();
    }
};

}  // namespace

TEST(AChildIsGivenItsHostByItsParentHybrid, ACandidateIsGivenTheHostAnsweredForItAndNoOther) {
    HybridParentHost host;
    Hy::static_child_host_hybrid machine(host);
    auto settle = [&machine] {
        for (int i = 0; i < 40; ++i) {
            machine.tick();
        }
    };
    machine.initialize();
    settle();

    ASSERT_EQ(host.asked, (Calls{"first"}));
    EXPECT_EQ(host.firsts[0].calls, (Calls{"first_ran"}));
    EXPECT_EQ(machine.completed(), 1u) << "it ran and ended";

    for (const char *name : {"again", "back"}) {
        machine.raiseExternal(name);
        settle();
    }
    ASSERT_EQ(host.asked, (Calls{"first", "second"}));
    EXPECT_EQ(host.seconds[0].calls, (Calls{"second_ran"}));
    // The first candidate's run is its own and was not repeated.
    EXPECT_EQ(host.firsts[0].calls, (Calls{"first_ran"}));
    EXPECT_EQ(machine.completed(), 2u);
}
