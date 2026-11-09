// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

/**
 * @file ManualVerificationTest.cpp
 * @brief The verdicts of the manual W3C fixtures, on runs written by hand
 *
 * The runner judges a manual fixture by a verifier that reads what the run
 * showed (ManualRun). A conformant engine only ever shows each verifier the
 * run it passes, so the full suite exercises the PASS branch of each and
 * nothing else: these cases hand each verifier the runs a defect would make
 * (a missing log group, an exit order reversed, an event processed after the
 * halt) and assert the verdict is not a pass. Each is a pure function of the
 * run, so a case costs no engine.
 */

#include "ManualVerification.h"
#include "impl/TestMetadataParser.h"
#include <algorithm>
#include <cctype>
#include <filesystem>
#include <gtest/gtest.h>
#include <set>
#include <string>
#include <vector>

namespace {

using SCE::W3C::ManualRun;
using SCE::W3C::TestResult;

ManualRun haltedIn(const std::string &state) {
    ManualRun run;
    run.loaded = true;
    run.halted = true;
    run.terminalState = state;
    run.endState = state;
    run.logCaptured = true;
    return run;
}

ManualRun stillRunningIn(const std::string &state) {
    ManualRun run;
    run.loaded = true;
    run.halted = false;
    run.endState = state;
    run.logCaptured = true;
    return run;
}

ManualRun rejectedAtLoad() {
    ManualRun run;
    run.loaded = false;
    run.loadError = "the document was refused";
    return run;
}

TestResult verdictOf(const std::string &fixtureId, const ManualRun &run) {
    const SCE::W3C::ManualVerifier *verifier = SCE::W3C::manualVerifierFor(fixtureId);
    EXPECT_NE(verifier, nullptr) << "no verifier for fixture " << fixtureId;
    if (verifier == nullptr) {
        return TestResult::ERROR;
    }
    return verifier->judge(run).finalResult;
}

// The messages one session logs for its copy of childToParent in test230
std::vector<std::string> eventFieldsLogged(const std::string &origin) {
    return {
        "name is : childToParent",
        "type is : external",
        "sendid is : ",
        "origin is : " + origin,
        "origintype is : http://www.w3.org/TR/scxml/#SCXMLEventProcessor",
        "invokeid is : s0.invoke",
        "data is : ",
    };
}

ManualRun with(ManualRun run, const std::vector<std::string> &messages) {
    run.logMessages.insert(run.logMessages.end(), messages.begin(), messages.end());
    return run;
}

// The fixtures whose metadata says they are manual, asked of the suite's own
// metadata through the parser the runner reads it with: which fixtures are
// manual is the registry's to say, and a list written here would be a copy of
// it that the next manual fixture leaves behind.
std::set<std::string> manualFixturesOfTheSuite() {
    namespace fs = std::filesystem;
    SCE::W3C::TestMetadataParser parser;
    std::set<std::string> ids;
    for (const auto &entry : fs::directory_iterator(SCE_W3C_RESOURCES_DIR)) {
        const std::string name = entry.path().filename().string();
        const fs::path metadata = entry.path() / "metadata.txt";
        if (!entry.is_directory() || name.empty() ||
            !std::all_of(name.begin(), name.end(), [](unsigned char c) { return std::isdigit(c) != 0; }) ||
            !fs::is_regular_file(metadata)) {
            continue;
        }
        if (parser.parseMetadata(metadata.string()).manual) {
            ids.insert(name);
        }
    }
    return ids;
}

TEST(ManualVerifierTable, HoldsEveryManualFixtureOfTheSuite) {
    std::set<std::string> ids;
    for (const auto &entry : SCE::W3C::manualVerifiers()) {
        ids.insert(entry.first);
        EXPECT_TRUE(entry.second.judge != nullptr) << entry.first;
    }
    const std::set<std::string> manual = manualFixturesOfTheSuite();
    // A suite read as having no manual fixture is a suite that was not read.
    ASSERT_FALSE(manual.empty()) << "no fixture under " << SCE_W3C_RESOURCES_DIR << " says it is manual";
    EXPECT_EQ(ids, manual);
}

TEST(ManualVerifierTable, AFixtureWithNoVerifierHasNone) {
    // The runner reports such a fixture as a skip under its own ID
    EXPECT_EQ(SCE::W3C::manualVerifierFor("144"), nullptr);
}

TEST(ManualLogReading, OnlyTheMessagesOfLogElementsAreRead) {
    const std::string captured = "[12:00:00.000] [info] log() - SCXML: name is : childToParent\n"
                                 "[12:00:00.001] [debug] executeLogAction() - Executing log action: \n"
                                 "[12:00:00.002] [error] loadSCXML() - Exception loading SCXML: boom\n"
                                 "[12:00:00.003] [info] log() - SCXML: \n"
                                 "[12:00:00.004] [info] log() - SCXML: Exiting sub01\n";
    EXPECT_EQ(SCE::W3C::logMessagesIn(captured),
              (std::vector<std::string>{"name is : childToParent", "", "Exiting sub01"}));
}

TEST(ManualLogReading, ALabelAloneReadsAsAnEmptyValue) {
    const std::vector<std::string> messages = {"no error in s0", "label: 1", "label: ", "labels: 2", "other"};
    EXPECT_EQ(SCE::W3C::valuesLoggedUnder(messages, "no error in s0"), (std::vector<std::string>{""}));
    EXPECT_EQ(SCE::W3C::valuesLoggedUnder(messages, "label"), (std::vector<std::string>{"1", ""}));
}

TEST(ManualVerdict178, FinalPassesAndAnythingElseFails) {
    EXPECT_EQ(verdictOf("178", haltedIn("final")), TestResult::PASS);
    EXPECT_EQ(verdictOf("178", haltedIn("fail")), TestResult::FAIL);
    EXPECT_EQ(verdictOf("178", stillRunningIn("s0")), TestResult::FAIL);
    EXPECT_EQ(verdictOf("178", rejectedAtLoad()), TestResult::ERROR);
}

TEST(ManualVerdict230, BothCopiesAlikePasses) {
    const std::vector<std::string> parent = eventFieldsLogged("#_scxml_child");
    EXPECT_EQ(verdictOf("230", with(with(haltedIn("final"), parent), parent)), TestResult::PASS);
}

TEST(ManualVerdict230, AFieldTheCopyChangedFails) {
    EXPECT_EQ(verdictOf("230", with(with(haltedIn("final"), eventFieldsLogged("#_scxml_child")),
                                    eventFieldsLogged("#_scxml_parent"))),
              TestResult::FAIL);
}

TEST(ManualVerdict230, AMissingGroupFails) {
    // The child never got the forwarded copy: one group only
    EXPECT_EQ(verdictOf("230", with(haltedIn("final"), eventFieldsLogged("#_scxml_child"))), TestResult::FAIL);
    EXPECT_EQ(verdictOf("230", haltedIn("final")), TestResult::FAIL);
}

TEST(ManualVerdict230, ARunThatDidNotReachFinalFails) {
    const std::vector<std::string> parent = eventFieldsLogged("#_scxml_child");
    EXPECT_EQ(verdictOf("230", with(with(haltedIn("fail"), parent), parent)), TestResult::FAIL);
    EXPECT_EQ(verdictOf("230", with(with(stillRunningIn("s02"), parent), parent)), TestResult::FAIL);
}

TEST(ManualVerdict230, NoCapturedLogIsAnErrorNotAVerdict) {
    ManualRun run = haltedIn("final");
    run.logCaptured = false;
    EXPECT_EQ(verdictOf("230", run), TestResult::ERROR);
}

TEST(ManualVerdict250, BothExitsInnermostFirstPasses) {
    EXPECT_EQ(verdictOf("250", with(haltedIn("final"), {"Exiting sub01", "Exiting sub0"})), TestResult::PASS);
}

TEST(ManualVerdict250, ExitsInTheWrongOrderFail) {
    EXPECT_EQ(verdictOf("250", with(haltedIn("final"), {"Exiting sub0", "Exiting sub01"})), TestResult::FAIL);
}

TEST(ManualVerdict250, AMissingExitFails) {
    EXPECT_EQ(verdictOf("250", with(haltedIn("final"), {"Exiting sub01"})), TestResult::FAIL);
    EXPECT_EQ(verdictOf("250", with(haltedIn("final"), {"Exiting sub0"})), TestResult::FAIL);
}

TEST(ManualVerdict250, ASessionThatWasNotCancelledFails) {
    EXPECT_EQ(verdictOf("250", with(haltedIn("final"), {"Exiting sub01", "Exiting sub0",
                                                        "entering final state, invocation was not cancelled"})),
              TestResult::FAIL);
}

TEST(ManualVerdict301, RejectionPassesAndARunFails) {
    EXPECT_EQ(verdictOf("301", rejectedAtLoad()), TestResult::PASS);
    EXPECT_EQ(verdictOf("301", haltedIn("fail")), TestResult::FAIL);
    EXPECT_EQ(verdictOf("301", haltedIn("pass")), TestResult::FAIL);
}

TEST(ManualVerdict307, TheSameValueAndNoErrorInBothPasses) {
    EXPECT_EQ(verdictOf("307", with(haltedIn("final"), {"entering s0 value of Var 1 is: : ", "no error in s0",
                                                        "entering s1, value of non-existent substructure of Var 1 "
                                                        "is: : ",
                                                        "No error in s1"})),
              TestResult::PASS);
}

TEST(ManualVerdict307, AnErrorInBothPasses) {
    // Neither value could be read, so neither was logged
    EXPECT_EQ(verdictOf("307", with(haltedIn("final"), {"error in state s0: {}", "error in state s1: {}"})),
              TestResult::PASS);
}

TEST(ManualVerdict307, DifferentValuesFail) {
    EXPECT_EQ(verdictOf("307", with(haltedIn("final"), {"entering s0 value of Var 1 is: : Var1", "no error in s0",
                                                        "entering s1, value of non-existent substructure of Var 1 "
                                                        "is: : Var1.foo",
                                                        "No error in s1"})),
              TestResult::FAIL);
}

TEST(ManualVerdict307, AnErrorInOneStateOnlyFails) {
    EXPECT_EQ(verdictOf("307", with(haltedIn("final"), {"error in state s0: {}",
                                                        "entering s1, value of non-existent substructure of Var 1 "
                                                        "is: : ",
                                                        "No error in s1"})),
              TestResult::FAIL);
}

TEST(ManualVerdict307, AMissingOutcomeFails) {
    EXPECT_EQ(verdictOf("307", with(haltedIn("final"), {"entering s0 value of Var 1 is: : ",
                                                        "entering s1, value of non-existent substructure of Var 1 "
                                                        "is: : ",
                                                        "No error in s1"})),
              TestResult::FAIL);
}

TEST(ManualVerdict313And314, RejectionOrPassPassesAndAnythingElseFails) {
    for (const char *fixtureId : {"313", "314"}) {
        EXPECT_EQ(verdictOf(fixtureId, rejectedAtLoad()), TestResult::PASS) << fixtureId;
        EXPECT_EQ(verdictOf(fixtureId, haltedIn("pass")), TestResult::PASS) << fixtureId;
        EXPECT_EQ(verdictOf(fixtureId, haltedIn("fail")), TestResult::FAIL) << fixtureId;
        EXPECT_EQ(verdictOf(fixtureId, stillRunningIn("s0")), TestResult::FAIL) << fixtureId;
    }
}

TEST(ManualVerdict415, HaltingWithNoEventProcessedPasses) {
    EXPECT_EQ(verdictOf("415", haltedIn("final")), TestResult::PASS);
}

TEST(ManualVerdict415, AnEventProcessedAfterTheHaltFails) {
    ManualRun run = haltedIn("final");
    run.eventsProcessed = 1;
    EXPECT_EQ(verdictOf("415", run), TestResult::FAIL);
}

TEST(ManualVerdict415, AMachineStillRunningFails) {
    EXPECT_EQ(verdictOf("415", stillRunningIn("final")), TestResult::FAIL);
}

}  // namespace
