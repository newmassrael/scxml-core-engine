// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Accepted Subset §2.15, "Format 2: a tree of sessions" (docs/adr/0045, section 8) and
// "Running host invocations" (section 3) — the C++ half of the Rust suite's
// `a_saved_tree_holds_every_session_where_it_stood.rs` and
// `a_child_sessions_host_invocation_is_served_by_the_trees_host.rs`: the tree fixture
// restored from the shared text, refused for a grandchild of another shape and a child of a
// later revision by the path of the session, the invokers a host registers on the machine it
// built serving every session of its tree, and an ending routed to the session that made the
// request.
//
// Fixture: `sce-build/tests/fixtures/static_datamodel/static_invoke_tree.scxml`. The shared
// text, `saved/static_invoke_tree_working.json`, is the tree `working` leaves: `worker` was
// started twice (`rest`, `back`), has taken `go`, holds a delayed `report` to its parent, asks
// its host for the time (a host-run invocation that has not answered), and runs a grandchild
// under the hybrid candidate `static_hybrid_watcher`, whose `pick` was changed to the other
// candidate after it started (`swap`). The parent holds a delayed `nudge` to the child (`arm`)
// and the child's `answer` in its queue; the child holds the parent's `swap` in its own.

#include "common/SceClock.h"
#include "core/HostProcessor.h"
#include "saved/SavedState.h"
#include "static_invoke_tree_sm.h"

#include <cstdint>
#include <filesystem>
#include <fstream>
#include <gtest/gtest.h>
#include <memory>
#include <optional>
#include <sstream>
#include <string>
#include <vector>

#ifndef SCE_PROJECT_ROOT
#define SCE_PROJECT_ROOT "."
#endif

namespace SCE {
namespace Tests {

namespace {

using Machine = ::SCE::Generated::static_invoke_tree::static_invoke_tree;

constexpr std::int64_t kSavedAtMs = 1'700'000'000'000;

/// The session id the shared format-2 fixtures give their root.
constexpr const char *kRoot = "fixture-root";
/// The session `worker` is: the root's id and the id its first start is started under
/// (`<state>.<start>.<invoke>`).
constexpr const char *kWorker = "fixture-root.working.1.worker";
/// The standard host type the child asks.
constexpr const char *kClock = "sce-host-clock-now";
/// A completion of the clock the child's result record reads.
constexpr const char *kNow = R"({"error":"","utc_ms":1700000000000,"monotonic_ms":5})";

std::string shared() {
    std::ifstream in(std::filesystem::path(SCE_PROJECT_ROOT) / "sce-build" / "tests" / "fixtures" / "static_datamodel" /
                     "saved" / "static_invoke_tree_working.json");
    std::stringstream held;
    held << in.rdbuf();
    std::string text = held.str();
    while (!text.empty() && (text.back() == '\n' || text.back() == '\r')) {
        text.pop_back();
    }
    return text;
}

/// Every start the host was handed, in order, and what it was told to stop.
struct Starts {
    std::vector<::SCE::HostInvokeRequest> requests;
    std::vector<::SCE::HostInvokeCancel> cancels;

    /// Register the clock on `machine`, the root, recording each start.
    void serve(Machine &machine) {
        machine.registerInvoker(kClock, [this](const ::SCE::HostInvokeEvent &event) {
            if (event.start.has_value()) {
                requests.push_back(*event.start);
            }
            if (event.cancel.has_value()) {
                cancels.push_back(*event.cancel);
            }
            return std::optional<::SCE::HostInvokeResponse>();
        });
    }
};

/// The rounds a host gives a tree after an event: each level of child session takes two.
void drive(Machine &machine) {
    for (int round = 0; round < 5; ++round) {
        machine.tick();
    }
}

std::unique_ptr<Machine> fresh() {
    auto machine = std::make_unique<Machine>();
    machine->setClock(std::make_shared<::SCE::ManualClock>(0));
    return machine;
}

/// A root built under the fixtures' id, served, and initialized.
std::unique_ptr<Machine> started(Starts &starts) {
    auto machine = fresh();
    machine->bindSessionId(kRoot);
    starts.serve(*machine);
    machine->initialize();
    drive(*machine);
    return machine;
}

void raise(Machine &machine, const std::string &name) {
    machine.raiseExternal(name);
    drive(machine);
}

std::unique_ptr<Machine> restored(const std::string &text) {
    auto machine = fresh();
    machine->restoreAt(::SCE::Saved::SavedState::fromJson(text), kSavedAtMs);
    return machine;
}

std::vector<std::string> actsOf(const std::vector<::SCE::Saved::SavedSend> &sends, ::SCE::Saved::SavedActKind kind) {
    std::vector<std::string> events;
    for (const auto &send : sends) {
        if (send.act.kind == kind) {
            events.push_back(send.act.event);
        }
    }
    return events;
}

std::vector<std::string> namesOf(const std::vector<::SCE::Saved::SavedEvent> &events) {
    std::vector<std::string> names;
    for (const auto &event : events) {
        names.push_back(event.name);
    }
    return names;
}

std::string refusalOf(const std::string &text) {
    try {
        static_cast<void>(restored(text));
    } catch (const ::SCE::Saved::StateRefusal &refused) {
        return refused.what();
    }
    return "";
}

}  // namespace

TEST(ASavedTreeHoldsEverySessionWhereItStood, TheTreeARunReachesHoldsEverySessionWhereItStood) {
    Starts starts;
    auto machine = started(starts);
    for (const char *name : {"rest", "back", "go", "swap", "arm"}) {
        raise(*machine, name);
    }
    const auto root = machine->saveAt(kSavedAtMs);
    EXPECT_EQ(root.sessionId, kRoot);
    EXPECT_EQ(root.current, "working");
    ASSERT_EQ(root.invokeStarts.size(), 1u);
    EXPECT_EQ(root.invokeStarts[0].id, "worker");
    EXPECT_EQ(root.invokeStarts[0].count, 2) << "the worker was started twice";
    EXPECT_EQ(actsOf(root.pending, ::SCE::Saved::SavedActKind::Child), std::vector<std::string>({"nudge"}))
        << "the parent holds a delayed send to the child";
    ASSERT_EQ(root.invokes.children.size(), 1u) << "one running child";
    const auto &worker = root.invokes.children[0];
    EXPECT_EQ(worker.id, "worker");
    EXPECT_EQ(worker.start, 2);
    EXPECT_EQ(worker.invokeId, "working.2.worker");
    const auto &child = *worker.session;
    EXPECT_EQ(child.sessionId, "fixture-root.working.2.worker");
    EXPECT_EQ(child.current, "second") << "the child took `go`";
    EXPECT_EQ(child.variable("steps").asNumber().text, "1");
    EXPECT_EQ(child.variable("pick").asString(), "static_hybrid_holder.scxml")
        << "`pick` was changed after the grandchild started";
    EXPECT_EQ(actsOf(child.pending, ::SCE::Saved::SavedActKind::Parent), std::vector<std::string>({"report"}))
        << "the child holds a delayed send to its parent";
    ASSERT_EQ(child.hostInvokes.size(), 1u);
    EXPECT_EQ(child.hostInvokes[0].processorType, kClock);
    EXPECT_EQ(child.hostInvokes[0].invokeId, "clock");

    ASSERT_EQ(child.invokes.children.size(), 1u) << "one grandchild";
    const auto &helper = child.invokes.children[0];
    EXPECT_EQ(helper.document, "static_hybrid_watcher") << "the candidate that was started, whatever `pick` names now";
    EXPECT_EQ(helper.start, 1);
    EXPECT_EQ(helper.invokeId.rfind("second.1.", 0), 0u) << helper.invokeId;
    EXPECT_EQ(child.variable("helper_id").asString(), helper.invokeId)
        << "the idlocation holds the id the grandchild was started under";
    EXPECT_EQ(starts.requests.size(), 1u) << "the child asked its host once, as it took `go`";
}

TEST(ASavedTreeHoldsEverySessionWhereItStood, TheRestoredTreeStandsWhereTheTextSays) {
    const auto root = restored(shared())->saveAt(kSavedAtMs);
    EXPECT_EQ(root.sessionId, kRoot);
    EXPECT_EQ(root.current, "working");
    EXPECT_EQ(namesOf(root.external), std::vector<std::string>({"answer"}))
        << "the child's answer waits in the parent's queue";
    EXPECT_EQ(actsOf(root.pending, ::SCE::Saved::SavedActKind::Child), std::vector<std::string>({"nudge"}));
    ASSERT_EQ(root.invokes.children.size(), 1u);
    const auto &child = *root.invokes.children[0].session;
    EXPECT_EQ(namesOf(child.external), std::vector<std::string>({"swap"}))
        << "the parent's `swap` waits in the child's queue";
    EXPECT_EQ(actsOf(child.pending, ::SCE::Saved::SavedActKind::Parent), std::vector<std::string>({"report"}));
}

TEST(ASavedTreeHoldsEverySessionWhereItStood, AGrandchildOfAnotherShapeRefusesTheTreeByItsPath) {
    std::string text = shared();
    const std::string shape = "a85ea43d8de628b004074e59c0ca90c198a4f7736073bd953750bca35c174c7a";
    const auto at = text.find(shape);
    ASSERT_NE(at, std::string::npos);
    text.replace(at, shape.size(), std::string(64, '0'));
    const std::string message = refusalOf(text);
    EXPECT_NE(message.find("invokes[0].session.invokes[0].session"), std::string::npos) << message;
    EXPECT_NE(message.find("shape"), std::string::npos) << message;
}

TEST(ASavedTreeHoldsEverySessionWhereItStood, AChildOfALaterRevisionRefusesTheTreeByItsPath) {
    // The child holds a record of `schema_revised.scxml`, saved at its third revision; a
    // fourth this build does not know is refused, naming the child.
    std::string text = shared();
    const std::string third = "\"revision\":3";
    const auto at = text.find(third);
    ASSERT_NE(at, std::string::npos) << "the child names the revision of its record";
    text.replace(at, third.size(), "\"revision\":4");
    const std::string message = refusalOf(text);
    EXPECT_NE(message.find("invokes[0].session"), std::string::npos) << message;
    EXPECT_NE(message.find("later than"), std::string::npos) << message;
}

TEST(ASavedTreeHoldsEverySessionWhereItStood, ASaveAfterAnyHostCallFindsNothingInTransit) {
    // After each call of a run that drives the child every way there is (a send made at
    // once, a delayed one armed, a forwarded event, a tick, a start again), the tree saves,
    // the text it saves restores, and the restored tree saves that text again.
    Starts starts;
    auto machine = started(starts);
    const auto check = [&machine](const std::string &after) {
        SCOPED_TRACE(after);
        const std::string text = machine->saveAt(kSavedAtMs).toJson();
        EXPECT_EQ(restored(text)->saveAt(kSavedAtMs).toJson(), text) << "the tree restored stands where it was saved";
    };
    for (const char *name : {"ask", "go", "ask", "swap", "ask", "arm", "ask", "rest", "back", "ask", "go", "ask"}) {
        machine->raiseExternal(name);
        check(std::string("raising `") + name + "`");
        drive(*machine);
        check(std::string("ticking after `") + name + "`");
    }
}

// ── a child's host-run invocation is served by the tree's host ─────────────

TEST(ASavedTreeHoldsEverySessionWhereItStood, AChildsInvocationReachesTheRootsInvokerAndIsEndedThroughTheRoot) {
    Starts starts;
    auto machine = started(starts);
    raise(*machine, "go");
    ASSERT_EQ(starts.requests.size(), 1u) << "one start";
    const auto request = starts.requests[0];
    EXPECT_EQ(request.processorType, kClock);
    EXPECT_EQ(request.invokeId, "clock");
    EXPECT_EQ(request.sessionId, kWorker) << "the request names the child session that made it";
    EXPECT_FALSE(request.restarted) << "a start the document makes is never a restart";

    EXPECT_EQ(machine->completeHostInvokeIn(kWorker, kClock, "clock", request.token, kNow),
              ::SCE::HostInvokeEndOutcome::Delivered);
    drive(*machine);
    EXPECT_EQ(machine->stamps(), 1u) << "the child took done.invoke.clock and told its parent";
}

TEST(ASavedTreeHoldsEverySessionWhereItStood, AnEndingForASessionTheTreeDoesNotHoldOrAStartItDoesNotRunIsRefused) {
    Starts starts;
    auto machine = started(starts);
    raise(*machine, "go");
    ASSERT_EQ(starts.requests.size(), 1u);
    const auto request = starts.requests[0];
    for (const char *nowhere : {"fixture-root.nowhere", "fixture-root.working.1.worker.nowhere", "elsewhere"}) {
        SCOPED_TRACE(nowhere);
        EXPECT_EQ(machine->completeHostInvokeIn(nowhere, kClock, "clock", request.token, kNow),
                  ::SCE::HostInvokeEndOutcome::NoSuchSession);
    }
    // The root runs no clock of its own.
    EXPECT_EQ(machine->completeHostInvokeIn(kRoot, kClock, "clock", request.token, kNow),
              ::SCE::HostInvokeEndOutcome::NotRunning);
    // A start ended once is ended: the second ending is stale.
    EXPECT_EQ(machine->completeHostInvokeIn(kWorker, kClock, "clock", request.token, kNow),
              ::SCE::HostInvokeEndOutcome::Delivered);
    EXPECT_EQ(machine->failHostInvokeIn(kWorker, kClock, "clock", request.token, "late"),
              ::SCE::HostInvokeEndOutcome::NotRunning);
    drive(*machine);
    EXPECT_EQ(machine->stamps(), 1u) << "the refused endings raised nothing";
}

TEST(ASavedTreeHoldsEverySessionWhereItStood, AResumedChildsInvocationStartsAgainAtItsFirstDrive) {
    Starts starts;
    auto machine = started(starts);
    raise(*machine, "go");
    ASSERT_EQ(starts.requests.size(), 1u);
    const auto first = starts.requests[0];

    auto again = restored(machine->saveAt(kSavedAtMs).toJson());
    // The host registers its invokers on the machine the restore handed it, after the
    // restore built the child: nothing has started yet.
    Starts served;
    served.serve(*again);
    EXPECT_TRUE(served.requests.empty()) << "a restore calls no host";

    // The parent's first drive of the child is the child's first macrostep.
    drive(*again);
    ASSERT_EQ(served.requests.size(), 1u);
    const auto request = served.requests[0];
    EXPECT_TRUE(request.restarted) << "started again from the saved request";
    EXPECT_EQ(request.sessionId, kWorker);
    EXPECT_EQ(request.invokeId, "clock");
    EXPECT_NE(request.token, first.token) << "a new token";

    EXPECT_EQ(again->completeHostInvokeIn(kWorker, kClock, "clock", request.token, kNow),
              ::SCE::HostInvokeEndOutcome::Delivered);
    drive(*again);
    EXPECT_EQ(again->stamps(), 1u);
}

TEST(ASavedTreeHoldsEverySessionWhereItStood, AnInvokeIdlocationKeepsTheIdTheGrandchildIsStartedUnder) {
    // docs/adr/0045: the grandchild is started under `second.1.<its invoke id>`, the
    // `invokeid` a saved tree holds for it, and `helper_id` holds the same.
    Starts starts;
    auto machine = started(starts);
    raise(*machine, "go");
    const auto saved = machine->saveAt(kSavedAtMs);
    ASSERT_EQ(saved.invokes.children.size(), 1u);
    const auto &worker = *saved.invokes.children[0].session;
    ASSERT_EQ(worker.invokes.children.size(), 1u) << "the hybrid candidate runs";
    const std::string startedUnder = worker.invokes.children[0].invokeId;
    EXPECT_EQ(startedUnder.rfind("second.1.", 0), 0u) << startedUnder;
    EXPECT_EQ(worker.variable("helper_id").asString(), startedUnder);
}

}  // namespace Tests
}  // namespace SCE
