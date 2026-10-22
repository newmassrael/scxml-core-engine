// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Accepted Subset §2.15, "Saving and restoring": a delayed `<send>` is a wait a saved
// machine carries, measured through the machine a generator wrote. Two kinds are held here,
// each the C++ half of the Rust, Kotlin, Go and Python suites:
//
//  * the id a machine generated for a `<send idlocation>` (`static_send_idlocation`;
//    `a_generated_send_id_survives_a_restore.rs`), counted by the machine and kept by the
//    variable that holds it, so a restored `<cancel>` finds its send and a send armed after
//    the restore is numbered on from the saved count;
//  * a send a HOST serves (`statechart_static_delayed_host_send`;
//    `a_generated_machines_waiting_host_send_survives_a_restore.rs`), saved with the request
//    it was armed with, so a restore hands the host the request the document made.
//
// Driven entirely on the engine's manual clock: no case sleeps. The shared texts are the
// format-2 `saved/static_send_idlocation_armed.json` and
// `saved/statechart_static_delayed_host_send_waiting.json` every backend writes
// (docs/adr/0045), the format-1 texts under `saved/format1/` of the same runs still restore.

#include "common/SceClock.h"
#include "core/HostProcessor.h"
#include "saved/SavedState.h"
#include "statechart_static_delayed_host_send_sm.h"
#include "static_send_idlocation_sm.h"

#include <cstdint>
#include <filesystem>
#include <fstream>
#include <gtest/gtest.h>
#include <memory>
#include <sstream>
#include <string>
#include <utility>
#include <vector>

#ifndef SCE_PROJECT_ROOT
#define SCE_PROJECT_ROOT "."
#endif

namespace SCE {
namespace Tests {

namespace {

using SendId = ::SCE::Generated::static_send_idlocation::static_send_idlocation;
using HostSend = ::SCE::Generated::statechart_static_delayed_host_send::statechart_static_delayed_host_send;

constexpr std::int64_t kSavedAtMs = 1'700'000'000'000;
constexpr const char *kRoot = "fixture-root";
constexpr const char *kDeclaredType = "x-sce-host";

std::string textOf(const std::filesystem::path &path) {
    std::ifstream in(path);
    std::stringstream held;
    held << in.rdbuf();
    std::string text = held.str();
    while (!text.empty() && (text.back() == '\n' || text.back() == '\r')) {
        text.pop_back();
    }
    return text;
}

std::string sharedArmed(bool format1 = false) {
    const auto saved =
        std::filesystem::path(SCE_PROJECT_ROOT) / "sce-build" / "tests" / "fixtures" / "static_datamodel" / "saved";
    return textOf((format1 ? saved / "format1" : saved) / "static_send_idlocation_armed.json");
}

std::string sharedWaiting(bool format1 = false) {
    const auto saved =
        std::filesystem::path(SCE_PROJECT_ROOT) / "sce-build" / "tests" / "fixtures" / "host_processor" / "saved";
    return textOf((format1 ? saved / "format1" : saved) / "statechart_static_delayed_host_send_waiting.json");
}

template <typename Machine> std::unique_ptr<Machine> fresh() {
    auto machine = std::make_unique<Machine>();
    machine->setClock(std::make_shared<::SCE::ManualClock>(0));
    return machine;
}

template <typename Machine> void send(Machine &machine, const std::string &name) {
    machine.raiseExternal(name);
    machine.advanceTimeMs(0);
}

/// A machine that has run `arm` once, on a clock the test owns.
std::unique_ptr<SendId> armed() {
    auto machine = fresh<SendId>();
    machine->bindSessionId(kRoot);
    machine->initialize();
    send(*machine, "arm");
    return machine;
}

std::unique_ptr<SendId> restoredSendId(const std::string &text) {
    auto machine = fresh<SendId>();
    machine->restoreAt(::SCE::Saved::SavedState::fromJson(text), kSavedAtMs);
    return machine;
}

}  // namespace

TEST(ASavedWaitingSendSurvivesARestore, AMachineThatArmedTwoSendsSavesTheIdsItGenerated) {
    const auto saved = armed()->saveAt(kSavedAtMs);
    EXPECT_EQ(saved.autoSendSeq, 2);
    EXPECT_EQ(saved.pending.size(), 2u) << "two ids were generated and two sends wait";
    const std::string text = saved.toJson();
    for (const char *id : {"_auto_send_1", "_auto_send_2"}) {
        EXPECT_NE(text.find(std::string("\"sendid\":\"") + id + "\""), std::string::npos)
            << "a waiting send carries " << id << ": " << text;
    }
    // The text every backend writes for the same run, byte for byte: the variables hold the
    // ids the waiting sends are known by.
    EXPECT_EQ(text, sharedArmed());
}

TEST(ASavedWaitingSendSurvivesARestore, ARestoredSendIsCancelledByTheIdTheDocumentHolds) {
    // From the format-2 text, and from the format-1 text of the same run.
    for (const bool format1 : {false, true}) {
        SCOPED_TRACE(format1 ? "format 1" : "format 2");
        auto machine = restoredSendId(sharedArmed(format1));
        send(*machine, "cancel.first");
        machine->advanceTimeMs(250);
        EXPECT_EQ(machine->first_fired(), 0u) << "its send was removed";
        EXPECT_EQ(machine->second_fired(), 1u) << "the other came due";
    }
}

TEST(ASavedWaitingSendSurvivesARestore, ARestoredMachineNumbersItsNextIdsAfterTheSavedCount) {
    auto machine = restoredSendId(sharedArmed());
    // The ids the saved state holds are 1 and 2: `arm` again writes 3 and 4 to the
    // variables, and a `<cancel>` by `first` removes the send of id 3 and not the one of id
    // 1, which is still waiting.
    send(*machine, "arm");
    const auto saved = machine->saveAt(kSavedAtMs);
    EXPECT_EQ(saved.autoSendSeq, 4) << "four ids were generated";
    EXPECT_NE(saved.toJson().find("\"first\":\"_auto_send_3\",\"second\":\"_auto_send_4\""), std::string::npos)
        << saved.toJson();

    send(*machine, "cancel.first");
    machine->advanceTimeMs(250);
    EXPECT_EQ(machine->first_fired(), 1u) << "the send of id 1 came due, the one of id 3 was removed";
    EXPECT_EQ(machine->second_fired(), 2u) << "ids 2 and 4 came due";
}

// ── a send the host serves ─────────────────────────────────────────────────

namespace {

/// What a restored machine's handler is asked: the engine's reading of "now" at the moment,
/// and the request.
struct Host {
    std::vector<std::pair<std::uint64_t, ::SCE::HostSendRequest>> calls;
    std::uint64_t now = 0;

    void registerOn(HostSend &machine) {
        machine.registerEventProcessor(kDeclaredType, [this](const ::SCE::HostSendRequest &request) {
            calls.emplace_back(now, request);
            return std::vector<::SCE::HostSendResponse>{};
        });
    }

    void advanceTo(HostSend &machine, std::uint64_t toMs) {
        ASSERT_GE(toMs, now) << "time does not run backwards in these cases";
        const std::uint64_t delta = toMs - now;
        now = toMs;
        machine.advanceTimeMs(delta);
    }
};

/// A machine that armed its send with `job` 7, saw `bump`, and has waited 100 of the 500 ms.
std::unique_ptr<HostSend> waiting() {
    auto machine = fresh<HostSend>();
    machine->bindSessionId(kRoot);
    machine->registerEventProcessor(
        kDeclaredType, [](const ::SCE::HostSendRequest &) { return std::vector<::SCE::HostSendResponse>{}; });
    machine->initialize();
    send(*machine, "start");
    send(*machine, "bump");
    machine->advanceTimeMs(100);
    return machine;
}

std::unique_ptr<HostSend> restoredHostSend(Host &host, const std::string &text, std::int64_t elapsedMs) {
    auto machine = fresh<HostSend>();
    machine->restoreAt(::SCE::Saved::SavedState::fromJson(text), kSavedAtMs + elapsedMs);
    host.registerOn(*machine);
    return machine;
}

std::string jobOf(const ::SCE::HostSendRequest &request) {
    const auto found = request.params.find("job");
    return found == request.params.end() || found->second.empty() ? "" : found->second.front();
}

}  // namespace

TEST(ASavedWaitingSendSurvivesARestore, AWaitingSendIsSavedWithTheRequestItWasArmedWith) {
    const auto saved = waiting()->saveAt(kSavedAtMs);
    ASSERT_EQ(saved.pending.size(), 1u) << "one send is waiting";
    const auto &entry = saved.pending[0];
    EXPECT_EQ(entry.due, kSavedAtMs + 500 - 100);
    const auto &act = entry.act;
    EXPECT_EQ(act.kind, ::SCE::Saved::SavedActKind::Host) << "a host-served send";
    EXPECT_EQ(act.processorType, kDeclaredType);
    EXPECT_EQ(act.event, "audit");
    EXPECT_EQ(act.target, "job://report");
    EXPECT_EQ(act.sendId, "a");
    // Armed with `job` 7: `bump` ran after, and the request does not follow it.
    EXPECT_EQ(act.params, (::SCE::Saved::SavedParams{{"job", {"7"}}, {"label", {"report"}}}));
    // The text every backend writes for the same machine, byte for byte. It also says the
    // machine itself carries the `job` it has now (8), apart from the 7 the request was
    // armed with.
    EXPECT_EQ(saved.toJson(), sharedWaiting());
}

TEST(ASavedWaitingSendSurvivesARestore, ARestoredSendIsMadeWhenItsMomentComesWithTheJobItWasArmedWith) {
    // Back 50 ms after the save: the send has 350 ms left of the 400 it had.
    Host host;
    auto machine = restoredHostSend(host, sharedWaiting(), 50);
    host.advanceTo(*machine, 349);
    EXPECT_TRUE(host.calls.empty()) << "nothing is due yet";
    host.advanceTo(*machine, 350);
    ASSERT_EQ(host.calls.size(), 1u) << "the send is made once";
    const auto &[at, request] = host.calls[0];
    EXPECT_EQ(at, 350u) << "at the moment it came due";
    EXPECT_EQ(request.processorType, kDeclaredType);
    EXPECT_EQ(request.eventName, "audit");
    EXPECT_EQ(request.target, "job://report");
    EXPECT_EQ(request.sendId, "a");
    EXPECT_EQ(jobOf(request), "7") << "the request the document made, not one evaluated again";
    EXPECT_EQ(request.eventData, "{\"job\":7,\"label\":\"report\"}");
}

TEST(ASavedWaitingSendSurvivesARestore, AWaitingSendSavedInFormatOneIsMadeWhenItsMomentComes) {
    Host host;
    auto machine = restoredHostSend(host, sharedWaiting(true), 50);
    host.advanceTo(*machine, 350);
    ASSERT_EQ(host.calls.size(), 1u);
    EXPECT_EQ(jobOf(host.calls[0].second), "7");
}

TEST(ASavedWaitingSendSurvivesARestore, ASendAlreadyDueWhenTheMachineComesBackIsMadeWhenItIsDriven) {
    // A minute away: the wait ran out while the process was dead.
    Host host;
    auto machine = restoredHostSend(host, sharedWaiting(), 60'000);
    EXPECT_TRUE(host.calls.empty()) << "restoring makes no send";
    host.advanceTo(*machine, 0);
    ASSERT_EQ(host.calls.size(), 1u);
    EXPECT_EQ(host.calls[0].first, 0u);
}

TEST(ASavedWaitingSendSurvivesARestore, ARestoredSendCanStillBeCancelledByItsId) {
    Host host;
    auto machine = restoredHostSend(host, sharedWaiting(), 0);
    send(*machine, "cancel");
    host.advanceTo(*machine, 10'000);
    EXPECT_TRUE(host.calls.empty()) << "the cancel found the send the saved state carried";
}

TEST(ASavedWaitingSendSurvivesARestore, ARestoredMachineCarriesOnWithTheJobItHad) {
    // `job` is 8 in the saved state; a `bump` after the restore makes it 9, and a new wait
    // armed from `idle` reads the machine's own field. The restored wait is over (at 400 ms)
    // before the second is armed.
    Host host;
    auto machine = restoredHostSend(host, sharedWaiting(), 0);
    host.advanceTo(*machine, 400);
    send(*machine, "stop");
    send(*machine, "bump");
    send(*machine, "start");
    host.advanceTo(*machine, 10'000);
    std::vector<std::string> jobs;
    for (const auto &call : host.calls) {
        jobs.push_back(jobOf(call.second));
    }
    EXPECT_EQ(jobs, (std::vector<std::string>{"7", "9"}))
        << "the restored wait first, then the one armed with the job the restored field held";
}

TEST(ASavedWaitingSendSurvivesARestore, ASavedStateRoundTripsThroughItsOwnText) {
    Host host;
    auto machine = restoredHostSend(host, sharedWaiting(), 0);
    EXPECT_EQ(machine->saveAt(kSavedAtMs).toJson(), sharedWaiting());
}

}  // namespace Tests
}  // namespace SCE
