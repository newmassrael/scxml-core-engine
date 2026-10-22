// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Accepted Subset §2.15, "Format 2: a tree of sessions" (docs/adr/0045, sections 1 and
// 3): a saved state holds each running child with its whole state, and a restore resumes it
// where it stood, in place of starting it again from its beginning, which is the one case in
// which a restore does not. The C++ half of `a_saved_child_session_is_resumed_after_a_restore.rs`,
// `ASavedChildSessionIsResumedAfterARestoreTest.kt`,
// `a_saved_child_session_is_resumed_after_a_restore_test.go` and
// `test_a_saved_child_session_is_resumed_after_a_restore.py`.
//
// Fixture: `static_invoke.scxml`; the shared text is `saved/static_invoke_working.json`, and
// its format-1 twin under `saved/format1/` names the child by its id alone.

#include "common/SceClock.h"
#include "saved/SavedState.h"
#include "static_invoke_sm.h"

#include <cstdint>
#include <filesystem>
#include <fstream>
#include <gtest/gtest.h>
#include <memory>
#include <sstream>
#include <string>

#ifndef SCE_PROJECT_ROOT
#define SCE_PROJECT_ROOT "."
#endif

namespace SCE {
namespace Tests {

namespace {

using Machine = ::SCE::Generated::static_invoke::static_invoke;
using State = Machine::PolicyType::State;

constexpr std::int64_t kSavedAtMs = 1'700'000'000'000;

std::string shared(bool format1 = false) {
    const auto saved =
        std::filesystem::path(SCE_PROJECT_ROOT) / "sce-build" / "tests" / "fixtures" / "static_datamodel" / "saved";
    std::ifstream in((format1 ? saved / "format1" : saved) / "static_invoke_working.json");
    std::stringstream held;
    held << in.rdbuf();
    std::string text = held.str();
    while (!text.empty() && (text.back() == '\n' || text.back() == '\r')) {
        text.pop_back();
    }
    return text;
}

std::unique_ptr<Machine> fresh() {
    auto machine = std::make_unique<Machine>();
    machine->setClock(std::make_shared<::SCE::ManualClock>(0));
    return machine;
}

/// The machine just started, its root given the id the shared fixtures name it by.
std::unique_ptr<Machine> started() {
    auto machine = fresh();
    machine->bindSessionId("fixture-root");
    machine->initialize();
    for (int round = 0; round < 3; ++round) {
        machine->tick();
    }
    return machine;
}

std::unique_ptr<Machine> restored(const std::string &text) {
    auto machine = fresh();
    machine->restoreAt(::SCE::Saved::SavedState::fromJson(text), kSavedAtMs);
    return machine;
}

/// Send `name` to the parent, which forwards it to its child, and let the child run.
void send(Machine &machine, const std::string &name) {
    machine.raiseExternal(name);
    for (int round = 0; round < 5; ++round) {
        machine.tick();
    }
}

bool working(const Machine &machine) {
    for (const State state : machine.getActiveStates()) {
        if (state == State::Working) {
            return true;
        }
    }
    return false;
}

/// The refusal a restore of `text` gives, or empty when there is none.
std::string refusalOf(const std::string &text) {
    try {
        static_cast<void>(restored(text));
    } catch (const ::SCE::Saved::StateRefusal &refused) {
        return refused.what();
    }
    return "";
}

/// `text` with the first `from` replaced by `to`, which must be there.
std::string replaced(const std::string &text, const std::string &from, const std::string &to) {
    const auto at = text.find(from);
    EXPECT_NE(at, std::string::npos) << from;
    std::string out = text;
    if (at != std::string::npos) {
        out.replace(at, from.size(), to);
    }
    return out;
}

}  // namespace

TEST(ASavedChildSessionIsResumedAfterARestore, AMachineIsSavedWithTheWholeStateOfTheChildItIsRunning) {
    const auto saved = started()->saveAt(kSavedAtMs);
    ASSERT_TRUE(saved.invokes.resumed);
    ASSERT_EQ(saved.invokes.children.size(), 1u);
    const auto &child = saved.invokes.children[0];
    EXPECT_EQ(child.id, "worker");
    EXPECT_EQ(child.start, 1);
    ASSERT_EQ(saved.invokeStarts.size(), 1u);
    EXPECT_EQ(saved.invokeStarts[0].id, "worker");
    EXPECT_EQ(saved.invokeStarts[0].count, 1);
    // Its id is this session's and the id it was started under: the tree's alone.
    EXPECT_EQ(child.invokeId, "working.1.worker");
    EXPECT_EQ(child.session->sessionId, "fixture-root.working.1.worker");
    EXPECT_EQ(child.session->current, "first");
    // The text every backend writes for the same machine, byte for byte.
    EXPECT_EQ(saved.toJson(), shared());
}

TEST(ASavedChildSessionIsResumedAfterARestore, AChildThatHadTakenAnEventIsResumedWithItTaken) {
    auto before = started();
    send(*before, "a");
    const auto saved = before->saveAt(kSavedAtMs);

    // Resumed at `second`: `b` alone ends it, and its `done.invoke` arrives once.
    auto after = restored(saved.toJson());
    EXPECT_TRUE(working(*after)) << "the restored machine stands in `working`";
    send(*after, "b");
    EXPECT_EQ(after->completed(), 1u) << "the child resumed where it stood and ended on `b`";
    send(*after, "b");
    EXPECT_EQ(after->completed(), 1u) << "it ended once";
}

TEST(ASavedChildSessionIsResumedAfterARestore, ARestoredTreeSavesTheTextItWasRestoredFrom) {
    EXPECT_EQ(restored(shared())->saveAt(kSavedAtMs).toJson(), shared())
        << "the child kept its id, its start and where it stood";
}

TEST(ASavedChildSessionIsResumedAfterARestore, TheCountOfStartsIsCarriedThroughARestore) {
    // The resumed child was not started again, and when it ends the count the restore
    // carried on is still its one start.
    auto before = started();
    send(*before, "a");
    auto after = restored(before->saveAt(kSavedAtMs).toJson());
    send(*after, "b");
    EXPECT_EQ(after->completed(), 1u);
    const auto again = after->saveAt(kSavedAtMs);
    ASSERT_EQ(again.invokeStarts.size(), 1u);
    EXPECT_EQ(again.invokeStarts[0].id, "worker");
    EXPECT_EQ(again.invokeStarts[0].count, 1);
    EXPECT_TRUE(again.invokes.invokeIds().empty()) << "the child ended, and is not listed";
}

TEST(ASavedChildSessionIsResumedAfterARestore, AChildThatHasEndedIsNotResumedOrStartedAgain) {
    // `done.invoke.worker` leaves the parent in `working`, so a machine saved after its
    // child ended stands in the state that invokes it with nothing running. The invocation
    // is complete: it is not listed, and a restore does not run it a second time.
    auto before = started();
    send(*before, "a");
    send(*before, "b");
    EXPECT_EQ(before->completed(), 1u);
    EXPECT_TRUE(working(*before));
    const auto saved = before->saveAt(kSavedAtMs);
    EXPECT_TRUE(saved.invokes.invokeIds().empty()) << "the child ended, and is not listed";

    auto after = restored(saved.toJson());
    EXPECT_EQ(after->completed(), 1u) << "the run is kept";
    send(*after, "a");
    send(*after, "b");
    EXPECT_EQ(after->completed(), 1u) << "a child started again would have ended again";
}

TEST(ASavedChildSessionIsResumedAfterARestore, AMachineThatLeftTheInvokingStateSavesNoInvocation) {
    auto machine = started();
    send(*machine, "abort");
    EXPECT_FALSE(working(*machine)) << "`abort` leaves `working`";
    const auto saved = machine->saveAt(kSavedAtMs);
    EXPECT_TRUE(saved.invokes.invokeIds().empty()) << "cancelled with its state";

    // And nothing is started when it comes back: no child is there to take `a` and `b`, and
    // `idle` invokes nothing.
    auto again = restored(saved.toJson());
    send(*again, "a");
    send(*again, "b");
    EXPECT_FALSE(again->isInFinalState());
}

TEST(ASavedChildSessionIsResumedAfterARestore, AnEventSavedInTheChildsQueueIsTakenByTheResumedChild) {
    // `a` reached the child's own queue and the child had not taken it when the tree was
    // saved: it is in the child's `external`, and the resumed child takes it, then ends on
    // `b`.
    const std::string text = shared();
    const std::string empty =
        "\"configuration\":[\"first\"],\"current\":\"first\",\"variables\":{},\"history\":{},\"pending\":[],"
        "\"invokes\":[],\"invokestarts\":{},\"hostinvokes\":[],\"hostinvoketoken\":\"0\",\"sendseq\":\"0\","
        "\"external\":[]";
    const std::string queued =
        replaced(empty, "\"external\":[]",
                 "\"external\":[{\"name\":\"a\",\"data\":\"\",\"type\":\"external\",\"sendid\":\"\",\"origin\":\"\","
                 "\"origintype\":\"\",\"invokeid\":\"\",\"hostinvoketoken\":null}]");
    auto machine = restored(replaced(text, empty, queued));
    for (int round = 0; round < 3; ++round) {
        machine->tick();
    }
    send(*machine, "b");
    EXPECT_EQ(machine->completed(), 1u) << "the queued `a` reached the resumed child";
}

TEST(ASavedChildSessionIsResumedAfterARestore, ATreeWhoseChildIsNotOneTheDocumentCouldRunIsRefusedWhole) {
    const struct {
        const char *what;
        const char *old;
        const char *replacement;
        const char *expected;
    } cases[] = {
        {"a child the document does not invoke", "\"id\":\"worker\"", "\"id\":\"stranger\"",
         "which the document does not invoke"},
        {"a start that is not the count its invoke holds", "\"start\":\"1\"", "\"start\":\"2\"",
         "the session counts 1 starts of it"},
        {"a child of a document its invoke does not start", "\"document\":\"static_invoke__sce_synth_invoke__worker\"",
         "\"document\":\"static_hybrid_holder\"", "does not start"},
        {"a child whose id is not its parent's and the id it was started under",
         "\"sessionid\":\"fixture-root.working.1.worker\"", "\"sessionid\":\"elsewhere\"",
         "a child of 'fixture-root' started under 'working.1.worker'"},
        {"a child of another shape", "688ad00d954ad9f35bdaa5eae06768a788173e0f8a8236e670efa68038781a6d",
         "0000000000000000000000000000000000000000000000000000000000000000",
         "the session at invokes[0].session: the saved state is of a document of shape"},
        {"a child that stands in no state of its document", "\"configuration\":[\"first\"],\"current\":\"first\"",
         "\"configuration\":[\"nowhere\"],\"current\":\"nowhere\"",
         "the session at invokes[0].session: the document has no state 'nowhere'"},
    };

    for (const auto &c : cases) {
        SCOPED_TRACE(c.what);
        const std::string message = refusalOf(replaced(shared(), c.old, c.replacement));
        EXPECT_NE(message.find(c.expected), std::string::npos) << "refused as " << message;
    }
}

// ── format 1: a child named by its id is started again ─────────────────────

TEST(ASavedChildSessionIsResumedAfterARestore, AFormatOneStateStartsItsChildAgainFromTheBeginning) {
    // The only case in which a child restarts: the text holds nothing of it.
    auto machine = restored(shared(true));
    EXPECT_TRUE(working(*machine)) << "the restored machine stands in `working`";

    // The child is at `first`: `b` is not what it waits for.
    send(*machine, "b");
    EXPECT_TRUE(working(*machine));
    EXPECT_EQ(machine->completed(), 0u) << "`b` alone does not end a child that starts over";

    send(*machine, "a");
    send(*machine, "b");
    EXPECT_EQ(machine->completed(), 1u) << "`a` then `b` ends it, and `done.invoke.worker` arrived once";
}

TEST(ASavedChildSessionIsResumedAfterARestore, AFormatOneStateNamingAnInvocationTheDocumentCannotHaveRunIsRefused) {
    const struct {
        const char *what;
        const char *old;
        const char *replacement;
        const char *expected;
    } cases[] = {
        {"an id the document does not invoke", "\"invokes\":[\"worker\"]", "\"invokes\":[\"stranger\"]",
         "which the document does not invoke"},
        {"an id named twice", "\"invokes\":[\"worker\"]", "\"invokes\":[\"worker\",\"worker\"]",
         "which an earlier entry already names"},
    };

    for (const auto &c : cases) {
        SCOPED_TRACE(c.what);
        const std::string message = refusalOf(replaced(shared(true), c.old, c.replacement));
        EXPECT_NE(message.find(c.expected), std::string::npos) << "refused as " << message;
    }
}

TEST(ASavedChildSessionIsResumedAfterARestore, AFormatOneInvocationWhoseStateTheConfigurationIsNotInIsRefused) {
    std::string text = replaced(shared(true), "\"configuration\":[\"working\"]", "\"configuration\":[\"idle\"]");
    text = replaced(text, "\"current\":\"working\"", "\"current\":\"idle\"");
    EXPECT_NE(refusalOf(text).find("whose state the saved configuration does not stand in"), std::string::npos);
}

}  // namespace Tests
}  // namespace SCE
