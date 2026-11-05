// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// docs/adr/0084 D5: the host-invoke ledger (`core/HostInvokeLedger.h`), apart from
// any engine. The scheduler is a table the cases read, and the invoker a handler
// that records what it was handed, so what the ledger arms, drops, asks and reports
// is seen directly rather than through the events an engine raises for it.

#include "core/HostInvokeLedger.h"

#include <cstdint>
#include <gtest/gtest.h>
#include <map>
#include <memory>
#include <optional>
#include <string>
#include <utility>
#include <variant>
#include <vector>

namespace SCE::Tests {

namespace {

using ::SCE::Core::HostInvokeEnded;
using ::SCE::Core::HostInvokeLedger;
using ::SCE::Core::HostInvokeNotStarted;
using ::SCE::Core::HostInvokeReport;
using ::SCE::Core::HostInvokeTimedOut;

/// The deadlines a ledger armed and has not dropped, by token, with the moment each
/// comes due: the scheduler half of the ledger's contract, as a table.
class RecordingDeadlines {
public:
    void scheduleHostInvokeDeadlineAt(uint64_t token, std::shared_ptr<const ::SCE::HostInvokeDeadline> deadline,
                                      uint64_t atMs) {
        armed[token] = std::make_pair(std::move(deadline), atMs);
    }

    void dropHostInvokeDeadline(uint64_t token) {
        armed.erase(token);
    }

    std::optional<uint64_t> hostInvokeDeadlineDueAt(uint64_t token) const {
        const auto it = armed.find(token);
        if (it == armed.end()) {
            return std::nullopt;
        }
        return it->second.second;
    }

    std::map<uint64_t, std::pair<std::shared_ptr<const ::SCE::HostInvokeDeadline>, uint64_t>> armed;
};

static_assert(::SCE::Core::HostInvokeDeadlineSchedulerTrait<RecordingDeadlines>::value,
              "the recording scheduler is not one a ledger can use");
static_assert(!::SCE::Core::HostInvokeDeadlineSchedulerTrait<int>::value, "a type with no deadline members serves");

/// What an invoker was handed, in order: the starts and the cancels.
struct Handed {
    std::vector<::SCE::HostInvokeRequest> starts;
    std::vector<::SCE::HostInvokeCancel> cancels;
};

/// An invoker that records what it is handed and answers every start with `answer`.
::SCE::HostInvokeHandler recording(Handed &handed, std::optional<::SCE::HostInvokeResponse> answer = std::nullopt) {
    return [&handed, answer](const ::SCE::HostInvokeEvent &event) -> std::optional<::SCE::HostInvokeResponse> {
        if (event.start.has_value()) {
            handed.starts.push_back(*event.start);
            return answer;
        }
        if (event.cancel.has_value()) {
            handed.cancels.push_back(*event.cancel);
        }
        return std::nullopt;
    };
}

::SCE::HostInvokeRequest request(const std::string &type, const std::string &id) {
    ::SCE::HostInvokeRequest made;
    made.processorType = type;
    made.invokeId = id;
    return made;
}

/// A clock that answers `atMs` and counts how often it was read.
struct CountingNow {
    uint64_t atMs = 0;
    int reads = 0;

    uint64_t operator()() {
        ++reads;
        return atMs;
    }
};

}  // namespace

// The start is recorded before the invoker is asked: a handler that ends the run
// before it returns ends a run the ledger already knows, and the start itself then
// has nothing left to report.
TEST(HostInvokeLedger, AStartIsRecordedBeforeItsHandlerIsAsked) {
    HostInvokeLedger ledger;
    RecordingDeadlines deadlines;
    ::SCE::TreeInvokers invokers;
    std::optional<HostInvokeEnded> endedInside;
    invokers.handlers["svc"] = [&](const ::SCE::HostInvokeEvent &event) -> std::optional<::SCE::HostInvokeResponse> {
        if (event.start.has_value()) {
            endedInside = ledger.endRun("svc", "job", event.start->token, false, "{}", std::nullopt, deadlines);
        }
        return std::nullopt;
    };
    CountingNow now;

    const auto report = ledger.start(invokers, request("svc", "job"), "root", deadlines, now);

    ASSERT_TRUE(report.has_value()) << "a registered invoker was reported as missing";
    EXPECT_TRUE(std::holds_alternative<std::monostate>(*report));
    ASSERT_TRUE(endedInside.has_value()) << "an ending made inside the handler found no running start";
    EXPECT_EQ(endedInside->invokeId, "job");
    EXPECT_EQ(endedInside->eventName(), "done.invoke.job");
}

// A completion the host returns with the start is handed back for the door a later
// one takes, with the run still recorded: that door ends it once, and the same token
// is refused afterwards.
TEST(HostInvokeLedger, AnImmediateCompletionEndsTheRunThroughTheOneDoor) {
    HostInvokeLedger ledger;
    RecordingDeadlines deadlines;
    ::SCE::TreeInvokers invokers;
    Handed handed;
    ::SCE::HostInvokeResponse done;
    done.doneData = "{\"ok\":true}";
    invokers.handlers["svc"] = recording(handed, done);
    CountingNow now;

    const auto report = ledger.start(invokers, request("svc", "job"), "root.child", deadlines, now);

    ASSERT_TRUE(report.has_value());
    const auto *completed = std::get_if<::SCE::Core::HostInvokeCompletedAtOnce>(&*report);
    ASSERT_NE(completed, nullptr) << "the completion returned with the start was not reported";
    EXPECT_EQ(completed->processorType, "svc");
    EXPECT_EQ(completed->invokeId, "job");
    EXPECT_EQ(completed->token, 0u);
    EXPECT_EQ(completed->doneData, "{\"ok\":true}");
    ASSERT_EQ(handed.starts.size(), 1u);
    EXPECT_EQ(handed.starts[0].sessionId, "root.child") << "the request does not name the session that started it";

    const auto ended = ledger.endRun(completed->processorType, completed->invokeId, completed->token, false,
                                     completed->doneData, std::nullopt, deadlines);
    ASSERT_TRUE(ended.has_value()) << "the run the host completed at once was not running for its door";
    EXPECT_FALSE(ended->failed);
    EXPECT_EQ(ended->data, "{\"ok\":true}");
    EXPECT_EQ(ended->eventName(), "done.invoke.job");
    EXPECT_FALSE(ledger.endRun("svc", "job", 0, false, "{}", std::nullopt, deadlines).has_value())
        << "a start ended twice";
    EXPECT_FALSE(ledger.cancel(invokers, "svc", "job", deadlines)) << "an ended start was cancelled";
    EXPECT_TRUE(handed.cancels.empty());
}

// A host that refuses the start reports the element's error with what it said; the
// start never ran, so its deadline is dropped, no cancel follows, and an ending
// reported for it later is refused.
TEST(HostInvokeLedger, ARefusedStartNeverRan) {
    HostInvokeLedger ledger;
    RecordingDeadlines deadlines;
    ::SCE::TreeInvokers invokers;
    Handed handed;
    ::SCE::HostInvokeResponse refused;
    refused.refusal = "not served here";
    refused.doneData = "{}";
    invokers.handlers["svc"] = recording(handed, refused);
    auto withDeadline = request("svc", "job");
    withDeadline.params[std::string(::SCE::HOST_INVOKE_DEADLINE_PARAM)] = {"100"};
    CountingNow now{5, 0};

    const auto report = ledger.start(invokers, withDeadline, "root", deadlines, now);

    ASSERT_TRUE(report.has_value());
    const auto *notStarted = std::get_if<HostInvokeNotStarted>(&*report);
    ASSERT_NE(notStarted, nullptr) << "a refusal was not reported as a start that did not happen";
    EXPECT_EQ(notStarted->reason, "not served here");
    EXPECT_TRUE(deadlines.armed.empty()) << "a refused start kept its deadline";
    EXPECT_FALSE(ledger.endRun("svc", "job", 0, false, "{}", std::nullopt, deadlines).has_value());
    EXPECT_FALSE(ledger.cancel(invokers, "svc", "job", deadlines));
    EXPECT_TRUE(handed.cancels.empty()) << "a refused start was told to stop";
}

// The deadline param is the engine's: it is taken out of what the host is handed and
// armed on the engine's clock. When it comes due on a running start, the host is told
// to stop and the report is the timeout; a completion after it is refused.
TEST(HostInvokeLedger, ADeadlineIsArmedFromItsParamAndEndsTheRunWhenItComesDue) {
    HostInvokeLedger ledger;
    RecordingDeadlines deadlines;
    ::SCE::TreeInvokers invokers;
    Handed handed;
    invokers.handlers["svc"] = recording(handed);
    auto withDeadline = request("svc", "job");
    withDeadline.params[std::string(::SCE::HOST_INVOKE_DEADLINE_PARAM)] = {"250"};
    withDeadline.params["kept"] = {"yes"};
    CountingNow now{1000, 0};

    const auto report = ledger.start(invokers, withDeadline, "root", deadlines, now);

    ASSERT_TRUE(report.has_value());
    EXPECT_TRUE(std::holds_alternative<std::monostate>(*report));
    ASSERT_EQ(handed.starts.size(), 1u);
    EXPECT_EQ(handed.starts[0].params.count(std::string(::SCE::HOST_INVOKE_DEADLINE_PARAM)), 0u)
        << "the host was handed the engine's deadline param";
    EXPECT_EQ(handed.starts[0].params.count("kept"), 1u);
    ASSERT_EQ(deadlines.hostInvokeDeadlineDueAt(0), std::optional<uint64_t>(1250));
    EXPECT_EQ(now.reads, 1);

    const auto saved = ledger.running(deadlines, now);
    ASSERT_EQ(saved.size(), 1u);
    EXPECT_EQ(saved[0].dueAtMs, std::optional<uint64_t>(1250)) << "a save does not read the pending deadline";

    const ::SCE::HostInvokeDeadline due = *deadlines.armed.at(0).first;
    deadlines.armed.erase(0);
    const HostInvokeReport expired = ledger.expire(invokers, due);
    const auto *timedOut = std::get_if<HostInvokeTimedOut>(&expired);
    ASSERT_NE(timedOut, nullptr) << "a deadline that came due on a running start did not end it";
    EXPECT_EQ(timedOut->eventName(), "error.invoke.job");
    EXPECT_EQ(HostInvokeTimedOut::data(), "\"deadline\"");
    ASSERT_EQ(handed.cancels.size(), 1u) << "the host was not told to stop a run whose deadline passed";
    EXPECT_EQ(handed.cancels[0].token, 0u);
    EXPECT_FALSE(ledger.endRun("svc", "job", 0, false, "{}", std::nullopt, deadlines).has_value())
        << "a completion after the deadline was taken";
}

// A deadline value that is not a whole number of milliseconds starts nothing, and the
// clock is not read for it. A type with no invoker is reported before it.
TEST(HostInvokeLedger, AnUnreadableDeadlineStartsNothingAndAMissingInvokerIsAskedFirst) {
    HostInvokeLedger ledger;
    RecordingDeadlines deadlines;
    ::SCE::TreeInvokers invokers;
    Handed handed;
    invokers.handlers["svc"] = recording(handed);
    auto badDeadline = request("svc", "job");
    badDeadline.params[std::string(::SCE::HOST_INVOKE_DEADLINE_PARAM)] = {"soon"};
    CountingNow now;

    const auto report = ledger.start(invokers, badDeadline, "root", deadlines, now);

    ASSERT_TRUE(report.has_value());
    const auto *notStarted = std::get_if<HostInvokeNotStarted>(&*report);
    ASSERT_NE(notStarted, nullptr);
    EXPECT_EQ(notStarted->reason,
              "<invoke type='svc' id='job'>: _sce_deadline_ms='soon' is not a whole number of milliseconds");
    EXPECT_TRUE(handed.starts.empty()) << "a start whose deadline cannot be read was handed to the host";
    EXPECT_EQ(now.reads, 0);

    auto unserved = request("other", "job");
    unserved.params[std::string(::SCE::HOST_INVOKE_DEADLINE_PARAM)] = {"soon"};
    EXPECT_FALSE(ledger.start(invokers, unserved, "root", deadlines, now).has_value())
        << "a type with no invoker was reported as started";
}

// A restart under the same id is a different process: it has a token of its own, and
// the first run's ending is stale.
TEST(HostInvokeLedger, ARestartMakesTheFirstRunsEndingStale) {
    HostInvokeLedger ledger;
    RecordingDeadlines deadlines;
    ::SCE::TreeInvokers invokers;
    Handed handed;
    invokers.handlers["svc"] = recording(handed);
    CountingNow now;

    ASSERT_TRUE(ledger.start(invokers, request("svc", "job"), "root", deadlines, now).has_value());
    ASSERT_TRUE(ledger.start(invokers, request("svc", "job"), "root", deadlines, now).has_value());

    ASSERT_EQ(handed.starts.size(), 2u);
    EXPECT_EQ(handed.starts[0].token, 0u);
    EXPECT_EQ(handed.starts[1].token, 1u);
    EXPECT_FALSE(ledger.endRun("svc", "job", 0, true, "late", std::nullopt, deadlines).has_value())
        << "the first run's ending was taken for the second";
    const auto ended =
        ledger.endRun("svc", "job", 1, true, "\"no\"", ::SCE::Core::EventOrigin("type", "peer"), deadlines);
    ASSERT_TRUE(ended.has_value());
    EXPECT_TRUE(ended->failed);
    EXPECT_EQ(ended->eventName(), "error.invoke.job");
    // docs/adr/0090: the pair the host named reaches the event whole.
    ASSERT_TRUE(ended->origin.has_value());
    EXPECT_EQ(ended->origin->address, "peer");
    EXPECT_EQ(ended->origin->processor, "type");
}

// What a restore re-armed starts at the next macrostep: one whose deadline passed
// while the machine was away is reported timed out without its host being asked, one
// whose invoker is not registered is the element's error, and the rest start as
// restarts with the deadline they kept. One cancelled before that is dropped and never
// told to stop.
TEST(HostInvokeLedger, RestoredInvocationsStartAtTheNextMacrostepOrEndWithoutAStart) {
    HostInvokeLedger ledger;
    RecordingDeadlines deadlines;
    ::SCE::TreeInvokers invokers;
    Handed handed;
    invokers.handlers["svc"] = recording(handed);
    ledger.continueTokensFrom(7);

    auto passed =
        HostInvokeLedger::RestoredHostInvoke{request("svc", "passed"), HostInvokeLedger::RestoredDeadline::Passed, 0};
    auto unregistered =
        HostInvokeLedger::RestoredHostInvoke{request("gone", "orphan"), HostInvokeLedger::RestoredDeadline::None, 0};
    auto armed =
        HostInvokeLedger::RestoredHostInvoke{request("svc", "armed"), HostInvokeLedger::RestoredDeadline::At, 500};
    armed.request.restarted = true;
    auto cancelled =
        HostInvokeLedger::RestoredHostInvoke{request("svc", "cancelled"), HostInvokeLedger::RestoredDeadline::None, 0};
    ledger.rearm(passed);
    ledger.rearm(unregistered);
    ledger.rearm(armed);
    ledger.rearm(cancelled);
    ASSERT_TRUE(ledger.hasRestored());

    CountingNow now{40, 0};
    const auto saved = ledger.running(deadlines, now);
    ASSERT_EQ(saved.size(), 4u);
    EXPECT_EQ(saved[0].request->processorType, "gone") << "a save is not ordered by type and id";
    EXPECT_EQ(saved[1].request->invokeId, "armed");
    EXPECT_EQ(saved[1].dueAtMs, std::optional<uint64_t>(500));
    EXPECT_EQ(saved[2].request->invokeId, "cancelled");
    EXPECT_FALSE(saved[2].dueAtMs.has_value());
    EXPECT_EQ(saved[3].request->invokeId, "passed");
    EXPECT_EQ(saved[3].dueAtMs, std::optional<uint64_t>(40)) << "a deadline that passed is not saved as due now";

    EXPECT_FALSE(ledger.cancel(invokers, "svc", "cancelled", deadlines));

    std::vector<HostInvokeReport> reports;
    ledger.startRestored(invokers, "root", deadlines,
                         [&reports](const HostInvokeReport &report) { reports.push_back(report); });

    EXPECT_FALSE(ledger.hasRestored());
    ASSERT_EQ(reports.size(), 3u) << "the cancelled restore was started, or one report was lost";
    const auto *timedOut = std::get_if<HostInvokeTimedOut>(&reports[0]);
    ASSERT_NE(timedOut, nullptr);
    EXPECT_EQ(timedOut->invokeId, "passed");
    const auto *notStarted = std::get_if<HostInvokeNotStarted>(&reports[1]);
    ASSERT_NE(notStarted, nullptr);
    EXPECT_EQ(notStarted->reason, "<invoke> names an invoker the host declared but never registered");
    EXPECT_TRUE(std::holds_alternative<std::monostate>(reports[2]));
    ASSERT_EQ(handed.starts.size(), 1u) << "a restore asked the host to start what had already ended";
    EXPECT_EQ(handed.starts[0].invokeId, "armed");
    EXPECT_TRUE(handed.starts[0].restarted);
    EXPECT_EQ(handed.starts[0].token, 7u) << "a restored start did not continue the saved tokens";
    EXPECT_EQ(ledger.nextToken(), 8u);
    EXPECT_EQ(deadlines.hostInvokeDeadlineDueAt(7), std::optional<uint64_t>(500));
    EXPECT_TRUE(handed.cancels.empty());
    EXPECT_TRUE(ledger.holds(::SCE::HostInvokeDeadline{"svc", "armed", 7}));
    EXPECT_FALSE(ledger.holds(::SCE::HostInvokeDeadline{"svc", "armed", 6}));
}

}  // namespace SCE::Tests
