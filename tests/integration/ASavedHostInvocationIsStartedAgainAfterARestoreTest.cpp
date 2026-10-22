// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Accepted Subset §2.15, "Saving and restoring": an `<invoke>` the HOST runs is part
// of what a machine is doing (§scxml-6.4.1), and the process that ran it is gone with the
// process that saved it. A saved state holds each running host-run invocation with the
// request it was started with and the deadline it had left, and a restore starts it again
// from that request, never evaluating the element a second time, with `restarted` set, a
// new token and what was left of the deadline.
//
// `statechart_static_host_invoke.scxml` hands a job to the host from `working`.
// `done.invoke.h` and `error.invoke.h` each leave it and record a digit in `seen`, so the
// number says which arrived. The Rust, Kotlin, Go and Python halves are
// `a_saved_host_invocation_is_started_again_after_a_restore.rs`,
// `ASavedHostInvocationIsStartedAgainAfterARestoreTest.kt`,
// `a_saved_host_invocation_is_started_again_after_a_restore_test.go` and
// `test_a_saved_host_invocation_is_started_again_after_a_restore.py`.
//
// Driven entirely on the engine's manual clock: no case sleeps, and none can be decided by
// how loaded the build machine is. The shared instance
// `saved/statechart_static_host_invoke_running.json` is the format-2 text every backend
// writes (docs/adr/0045); the format-1 text of the same run, under `saved/format1/`, still
// restores. This is a machine with no `error.execution` event, so a restart nobody can run
// raises nothing a document could answer, which the Python half's last case needs.

#include "common/SceClock.h"
#include "core/HostProcessor.h"
#include "saved/SavedState.h"
#include "statechart_static_host_invoke_sm.h"

#include <cstdint>
#include <filesystem>
#include <fstream>
#include <gtest/gtest.h>
#include <memory>
#include <optional>
#include <set>
#include <sstream>
#include <string>
#include <vector>

#ifndef SCE_PROJECT_ROOT
#define SCE_PROJECT_ROOT "."
#endif

namespace SCE {
namespace Tests {

namespace {

using Machine = ::SCE::Generated::statechart_static_host_invoke::statechart_static_host_invoke;
using State = Machine::PolicyType::State;

constexpr const char *kHostType = "x-sce-host";

/// The wall-clock moment the shared instance was saved at.
constexpr std::int64_t kSavedAtMs = 1'700'000'000'000;

const std::filesystem::path kSaved =
    std::filesystem::path(SCE_PROJECT_ROOT) / "sce-build" / "tests" / "fixtures" / "host_processor" / "saved";

std::string shared(bool format1 = false) {
    std::ifstream in((format1 ? kSaved / "format1" : kSaved) / "statechart_static_host_invoke_running.json");
    std::stringstream held;
    held << in.rdbuf();
    std::string text = held.str();
    while (!text.empty() && (text.back() == '\n' || text.back() == '\r')) {
        text.pop_back();
    }
    return text;
}

/// What the host was told, in order: a start or a cancel.
struct Host {
    struct Entry {
        bool start;
        ::SCE::HostInvokeRequest request;
        ::SCE::HostInvokeCancel cancel;
    };

    std::vector<Entry> log;

    void registerOn(Machine &machine) {
        machine.registerInvoker(kHostType, [this](const ::SCE::HostInvokeEvent &event) {
            if (event.start.has_value()) {
                log.push_back({true, *event.start, {}});
            }
            if (event.cancel.has_value()) {
                log.push_back({false, {}, *event.cancel});
            }
            return std::optional<::SCE::HostInvokeResponse>();
        });
    }

    std::vector<::SCE::HostInvokeRequest> starts() const {
        std::vector<::SCE::HostInvokeRequest> started;
        for (const Entry &entry : log) {
            if (entry.start) {
                started.push_back(entry.request);
            }
        }
        return started;
    }
};

/// A machine on a manual clock, which a step moves on.
std::unique_ptr<Machine> fresh() {
    auto machine = std::make_unique<Machine>();
    machine->setClock(std::make_shared<::SCE::ManualClock>(0));
    return machine;
}

/// A machine standing in `working`, its invocation started with token 0, given the session
/// id the shared format-2 fixtures name their root by.
std::unique_ptr<Machine> working(Host &host) {
    auto machine = fresh();
    machine->bindSessionId("fixture-root");
    host.registerOn(*machine);
    machine->initialize();
    machine->raiseExternal("start");
    machine->advanceTimeMs(0);
    return machine;
}

/// `text` restored `elapsedMs` of wall-clock time after it was saved, with no host yet.
std::unique_ptr<Machine> tryRestore(const std::string &text, std::int64_t elapsedMs) {
    auto machine = fresh();
    machine->restoreAt(::SCE::Saved::SavedState::fromJson(text), kSavedAtMs + elapsedMs);
    return machine;
}

/// `text` restored `elapsedMs` after it was saved, its host invoker registered the way a
/// host does it: on the machine the restore returned.
std::unique_ptr<Machine> restored(Host &host, const std::string &text, std::int64_t elapsedMs) {
    auto machine = tryRestore(text, elapsedMs);
    host.registerOn(*machine);
    return machine;
}

std::set<State> standingIn(const Machine &machine) {
    const auto active = machine.getActiveStates();
    return std::set<State>(active.begin(), active.end());
}

}  // namespace

TEST(ASavedHostInvocationIsStartedAgainAfterARestore, AMachineIsSavedWithTheRequestItsInvocationWasStartedWith) {
    Host host;
    auto machine = working(host);
    const auto first = host.starts().at(0);
    EXPECT_FALSE(first.restarted) << "the document's own start is not a restart";
    EXPECT_EQ(first.token, 0u);
    EXPECT_EQ(first.sessionId, "fixture-root");

    const ::SCE::Saved::SavedState saved = machine->saveAt(kSavedAtMs);
    ASSERT_EQ(saved.hostInvokes.size(), 1u) << "one running invocation";
    const auto &entry = saved.hostInvokes[0];
    EXPECT_EQ(entry.processorType, kHostType);
    EXPECT_EQ(entry.invokeId, "h");
    EXPECT_EQ(entry.src, first.src);
    EXPECT_EQ(entry.content, first.content);
    EXPECT_EQ(entry.data, first.eventData);
    // The deadline is the invocation's own, not a send waiting to be delivered.
    EXPECT_EQ(entry.due, kSavedAtMs + 5000);
    EXPECT_TRUE(saved.pending.empty());
    // One start has been made, so the next token is 1.
    EXPECT_EQ(saved.hostInvokeToken, 1);
    // The text every backend writes for the same machine, byte for byte.
    EXPECT_EQ(saved.toJson(), shared());
}

TEST(ASavedHostInvocationIsStartedAgainAfterARestore, ARestoreAsksNothingOfTheHostUntilTheMachineIsDriven) {
    // From the format-2 text, and from the format-1 text of the same run.
    for (const bool format1 : {false, true}) {
        SCOPED_TRACE(format1 ? "format 1" : "format 2");
        Host host;
        auto machine = restored(host, shared(format1), 1000);
        EXPECT_TRUE(host.log.empty()) << "the restore itself calls nobody";
        EXPECT_EQ(standingIn(*machine), std::set<State>({State::Working}));
        // The first macrostep starts it, where an invocation the document entered would
        // have started.
        machine->advanceTimeMs(0);
        EXPECT_EQ(host.starts().size(), 1u);
    }
}

TEST(ASavedHostInvocationIsStartedAgainAfterARestore, ARestoredInvocationIsStartedAgainFromTheRequestItWasSavedWith) {
    Host original;
    auto first = working(original);
    const auto firstRequest = original.starts().at(0);

    Host host;
    auto machine = restored(host, shared(), 1000);
    machine->advanceTimeMs(0);
    ASSERT_EQ(host.starts().size(), 1u);
    const auto again = host.starts()[0];
    EXPECT_TRUE(again.restarted) << "the host is told this is a resumption";
    EXPECT_EQ(again.token, 1u) << "the next token the saved state carried on";
    EXPECT_EQ(again.processorType, firstRequest.processorType);
    EXPECT_EQ(again.invokeId, firstRequest.invokeId);
    EXPECT_EQ(again.src, firstRequest.src);
    EXPECT_EQ(again.params, firstRequest.params);
    EXPECT_EQ(again.eventData, firstRequest.eventData);
    EXPECT_EQ(again.content, firstRequest.content) << "the same request, not one evaluated again";
}

TEST(ASavedHostInvocationIsStartedAgainAfterARestore, ARestoredInvocationKeepsTheDeadlineItHadLeft) {
    // Saved with 5000 ms to run, restored 1000 ms later: 4000 ms are left.
    Host host;
    auto machine = restored(host, shared(), 1000);
    machine->advanceTimeMs(0);
    machine->advanceTimeMs(3999);
    EXPECT_EQ(machine->seen(), 0u) << "not due yet";
    EXPECT_EQ(standingIn(*machine), std::set<State>({State::Working}));
    machine->advanceTimeMs(1);
    EXPECT_EQ(machine->seen(), 2u) << "error.invoke.h, at 4000 ms";
    EXPECT_EQ(standingIn(*machine), std::set<State>({State::Failed}));
    // The host is told to stop what it was given, with the token it was given.
    ASSERT_FALSE(host.log.empty());
    const auto &last = host.log.back();
    EXPECT_FALSE(last.start);
    EXPECT_EQ(last.cancel.token, 1u);
    EXPECT_EQ(last.cancel.invokeId, "h");
}

TEST(ASavedHostInvocationIsStartedAgainAfterARestore,
     ADeadlineThatPassedWhileTheMachineWasAwayEndsTheInvocationUnstarted) {
    // 6 s after the save, and the deadline was at 5 s.
    Host host;
    auto machine = restored(host, shared(), 6000);
    machine->advanceTimeMs(0);
    EXPECT_TRUE(host.log.empty()) << "the host is not asked to begin what it would be told to stop";
    EXPECT_EQ(machine->seen(), 2u) << "the document hears error.invoke.h";
    EXPECT_EQ(standingIn(*machine), std::set<State>({State::Failed}));
}

TEST(ASavedHostInvocationIsStartedAgainAfterARestore, ARestoredMachineOwingAStartSaysItNeedsATickNow) {
    // A host drives a machine by asking when it next needs a tick and sleeping that long
    // (`timeUntilNextScheduled`). A restored invocation owes the machine a tick before
    // anything else: it is started in the first macrostep, and the deadline it keeps is
    // armed there. A machine that answered "nothing" would be left asleep for good, holding
    // an invocation no one had started.
    for (const std::int64_t elapsedMs : {1000, 6000}) {
        SCOPED_TRACE(elapsedMs);
        Host host;
        auto machine = restored(host, shared(), elapsedMs);
        const auto wait = machine->timeUntilNextScheduled();
        ASSERT_TRUE(wait.has_value());
        EXPECT_EQ(wait->count(), 0);
        machine->advanceTimeMs(0);
    }
}

TEST(ASavedHostInvocationIsStartedAgainAfterARestore, WhatTheFirstRunHandedAHostCannotAnswerForTheSecond) {
    // Token 0 belonged to the process that saved the machine. A late reply carrying it is a
    // different process's, and is ignored (§scxml-6.4).
    Host host;
    auto machine = restored(host, shared(), 1000);
    machine->advanceTimeMs(0);
    const std::uint64_t token = host.starts().at(0).token;
    EXPECT_FALSE(machine->completeHostInvoke(kHostType, "h", 0, "late")) << "a reply of the first run is accepted";
    machine->advanceTimeMs(0);
    EXPECT_EQ(machine->seen(), 0u);
    EXPECT_EQ(standingIn(*machine), std::set<State>({State::Working}));
    EXPECT_TRUE(machine->completeHostInvoke(kHostType, "h", token, "done")) << "the reply of this run is refused";
    machine->advanceTimeMs(0);
    EXPECT_EQ(machine->seen(), 1u);
    EXPECT_EQ(standingIn(*machine), std::set<State>({State::After}));
}

TEST(ASavedHostInvocationIsStartedAgainAfterARestore,
     ACompletionStillQueuedWhenTheMachineWasSavedIsAcceptedAfterTheRestore) {
    Host host;
    auto machine = working(host);
    // The host answers; the completion is on the external queue, stamped with the start's
    // token, and the machine has not been driven through it.
    ASSERT_TRUE(machine->completeHostInvoke(kHostType, "h", 0, "done")) << "the completion is accepted";
    const ::SCE::Saved::SavedState saved = machine->saveAt(kSavedAtMs);
    EXPECT_TRUE(saved.hostInvokes.empty()) << "the invocation ended";
    ASSERT_EQ(saved.external.size(), 1u) << "one queued completion carrying its token";
    EXPECT_EQ(saved.external[0].name, "done.invoke.h");
    EXPECT_EQ(saved.external[0].hostInvokeToken, std::optional<std::int64_t>(0));
    ASSERT_EQ(host.log.size(), 1u) << "started once, never cancelled";
    EXPECT_TRUE(host.log[0].start);

    Host againHost;
    auto again = restored(againHost, saved.toJson(), 0);
    again->advanceTimeMs(0);
    EXPECT_EQ(again->seen(), 1u) << "taken once, not refused";
    EXPECT_EQ(standingIn(*again), std::set<State>({State::After}));
    EXPECT_TRUE(againHost.log.empty()) << "an ended invocation is not started again";
    EXPECT_EQ(again->refusedHostInvokeCompletions(), 0u) << "the completion was refused";
}

TEST(ASavedHostInvocationIsStartedAgainAfterARestore,
     ACompletionTheEngineWasAboutToRefuseIsStillRefusedAfterTheRestore) {
    // A completion that carries no token did not come through `completeHostInvoke`, so the
    // engine refuses it when it is dequeued, because it may be a cancelled run's reply.
    // Another backend's state may queue one, and the restored machine refuses it as that
    // one would.
    const std::string sharedText = shared();
    const std::string queued =
        "\"external\":[{\"name\":\"done.invoke.h\",\"data\":\"\",\"type\":\"external\",\"sendid\":\"\",\"origin\":\"\","
        "\"origintype\":\"\",\"invokeid\":\"h\",\"hostinvoketoken\":null}]";
    const std::string empty = "\"external\":[]";
    std::string text = sharedText;
    const auto at = text.find(empty);
    ASSERT_NE(at, std::string::npos);
    text.replace(at, empty.size(), queued);
    ASSERT_FALSE(::SCE::Saved::SavedState::fromJson(text).external[0].hostInvokeToken.has_value())
        << "the event carries no token";

    Host host;
    auto again = restored(host, text, 0);
    again->advanceTimeMs(0);
    EXPECT_EQ(again->refusedHostInvokeCompletions(), 1u);
    EXPECT_EQ(again->seen(), 0u);
    EXPECT_EQ(standingIn(*again), std::set<State>({State::Working}));
}

TEST(ASavedHostInvocationIsStartedAgainAfterARestore, AnInvocationCancelledBeforeItWasStartedAgainIsNeverStarted) {
    Host host;
    auto machine = restored(host, shared(), 1000);
    // Nothing was started, so there is nothing to tell the host to stop.
    EXPECT_FALSE(machine->cancelHostInvoke(kHostType, "h")) << "there is no run to cancel";
    machine->advanceTimeMs(0);
    EXPECT_TRUE(host.log.empty());
}

TEST(ASavedHostInvocationIsStartedAgainAfterARestore,
     AMachineRestoredAndSavedAgainBeforeItWasDrivenKeepsItsInvocation) {
    Host host;
    auto machine = restored(host, shared(), 1000);
    // The same request, due at the same wall-clock moment, and the same counter.
    EXPECT_EQ(machine->saveAt(kSavedAtMs + 1000).toJson(), shared());
}

TEST(ASavedHostInvocationIsStartedAgainAfterARestore,
     ASavedStateNamingAHostInvocationTheDocumentCannotHaveRunIsRefused) {
    const struct {
        const char *what;
        const char *old;
        const char *replacement;
    } cases[] = {
        {"an id the document does not invoke", "\"id\":\"h\"", "\"id\":\"stranger\""},
        {"a type the document does not hand that id to", "\"type\":\"x-sce-host\"", "\"type\":\"x-other\""},
    };

    for (const auto &c : cases) {
        SCOPED_TRACE(c.what);
        std::string text = shared();
        const auto at = text.find(c.old);
        ASSERT_NE(at, std::string::npos);
        text.replace(at, std::string(c.old).size(), c.replacement);
        try {
            static_cast<void>(tryRestore(text, 0));
            FAIL() << "the restore was not refused";
        } catch (const ::SCE::Saved::StateRefusal &refused) {
            EXPECT_NE(std::string(refused.what()).find("which the document does not have a host invoker run"),
                      std::string::npos)
                << refused.what();
        }
    }
}

TEST(ASavedHostInvocationIsStartedAgainAfterARestore, AHostInvocationWhoseStateTheConfigurationIsNotInIsRefused) {
    std::string text = shared();
    const auto replaceOnce = [&text](const std::string &old, const std::string &replacement) {
        const auto at = text.find(old);
        ASSERT_NE(at, std::string::npos);
        text.replace(at, old.size(), replacement);
    };
    replaceOnce("\"configuration\":[\"working\"]", "\"configuration\":[\"idle\"]");
    replaceOnce("\"current\":\"working\"", "\"current\":\"idle\"");
    try {
        static_cast<void>(tryRestore(text, 0));
        FAIL() << "the restore was not refused";
    } catch (const ::SCE::Saved::StateRefusal &refused) {
        EXPECT_NE(std::string(refused.what()).find("whose state the saved configuration does not stand in"),
                  std::string::npos)
            << refused.what();
    }
}

TEST(ASavedHostInvocationIsStartedAgainAfterARestore, AnInvocationNamedTwiceIsRefused) {
    const std::string text = shared();
    const std::string open = "\"hostinvokes\":[";
    const auto start = text.find(open) + open.size();
    const auto end = text.find("],\"hostinvoketoken\"", start);
    ASSERT_NE(end, std::string::npos);
    const std::string one = text.substr(start, end - start);
    const std::string twice = text.substr(0, start) + one + "," + one + text.substr(end);
    try {
        static_cast<void>(tryRestore(twice, 0));
        FAIL() << "the restore was not refused";
    } catch (const ::SCE::Saved::StateRefusal &refused) {
        EXPECT_NE(std::string(refused.what()).find("which an earlier entry already names"), std::string::npos)
            << refused.what();
    }
}

}  // namespace Tests
}  // namespace SCE
