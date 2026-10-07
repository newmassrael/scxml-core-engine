// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// datamodel="sce-static" (docs/SCE_ACCEPTED_SUBSET.md §2.15), lowered to the
// ecmascript document the Interpreter runs, and run.
//
// The Interpreter refuses a `sce-static` document as written
// (AStaticDatamodelRunsUnderTheInterpreterTest): its script engine holds no
// types and checks no integer operation, so it would run the document with a
// meaning the document did not give it. `sce-codegen lower` is the other
// answer — it replaces the document's expressions with ECMAScript that means
// what the typed document means, and every other byte is the author's.
//
// What this holds it to is the one thing that makes the lowering worth having:
// the Interpreter, run on the lowered document, does what the generated
// backends do. The Kotlin and Rust backends replay
// `sce-build/tests/fixtures/static_datamodel/scenarios/*.json` — each a list
// of steps, an external event and what the machine must hold after it — and
// this replays the same files. One oracle judges three engines, so a
// disagreement is a defect in one of them and not a difference in what each
// was asked.
//
// Nothing here is listed. The scenarios are found by scanning the directory,
// the machine each one drives by the `machine` it names, and whether the
// Interpreter can run it by asking `sce-codegen lower`: a machine whose
// constructs have no lowering yet is refused with a typed diagnostic naming
// the construct, and starts being replayed the day the lowering covers it.

#include "events/EventDispatcherImpl.h"
#include "events/EventSchedulerImpl.h"
#include "events/EventTargetFactoryImpl.h"
#include "runtime/EventRaiserImpl.h"
#include "runtime/INativeActionHost.h"
#include "runtime/StateMachine.h"
#include "scripting/ScriptEngineProvider.h"

#include <algorithm>
#include <chrono>
#include <cstdio>
#include <cstdlib>
#include <filesystem>
#include <fstream>
#include <functional>
#include <gtest/gtest.h>
#include <memory>
#include <nlohmann/json.hpp>
#include <sstream>
#include <string>
#include <thread>
#include <unistd.h>
#include <vector>

#ifndef SCE_PROJECT_ROOT
#define SCE_PROJECT_ROOT "."
#endif

namespace SCE {
namespace Tests {

namespace {

const std::filesystem::path kFixtures =
    std::filesystem::path(SCE_PROJECT_ROOT) / "sce-build/tests/fixtures/static_datamodel";

std::string slurp(const std::filesystem::path &path) {
    std::ifstream in(path);
    EXPECT_TRUE(in.is_open()) << "not readable: " << path;
    std::ostringstream buffer;
    buffer << in.rdbuf();
    return buffer.str();
}

/// What a command wrote to the stream it was asked to keep, and how it ended.
struct Ran {
    int status = -1;
    std::string output;
};

Ran capture(const std::string &command) {
    Ran ran;
    FILE *pipe = popen(command.c_str(), "r");
    if (pipe == nullptr) {
        return ran;
    }
    char buffer[4096];
    while (std::fgets(buffer, sizeof(buffer), pipe) != nullptr) {
        ran.output += buffer;
    }
    ran.status = pclose(pipe);
    return ran;
}

/// `sce-codegen lower <fixture>`: the lowered document on stdout when it
/// succeeds; when it refuses, the diagnostic, from the stream diagnostics
/// travel on.
struct Lowered {
    bool ok = false;
    std::string document;
    nlohmann::json refusal;
};

Lowered lower(const std::filesystem::path &fixture) {
    const char *bin = std::getenv("SCE_CODEGEN_BIN");
    EXPECT_NE(bin, nullptr) << "SCE_CODEGEN_BIN must be set by CMake add_test ENVIRONMENT";
    Lowered lowered;
    if (bin == nullptr) {
        return lowered;
    }
    const std::string base = std::string("\"") + bin + "\" lower \"" + fixture.string() + "\" --error-format=json";
    const Ran ran = capture(base + " 2>/dev/null");
    if (ran.status == 0) {
        lowered.ok = true;
        lowered.document = ran.output;
        return lowered;
    }
    // The diagnostic is one NDJSON record on stderr; stdout is discarded so
    // that what is read here is what the refusal said.
    const Ran refused = capture(base + " 2>&1 >/dev/null");
    const auto eol = refused.output.find('\n');
    lowered.refusal = nlohmann::json::parse(refused.output.substr(0, eol), nullptr, false);
    return lowered;
}

/// `sce-codegen lower <fixture> --out-dir <dir>`: the lowered document and the
/// documents its hybrid `<invoke>`s may start, written into `dir`, and the paths
/// the one JSON line on stdout names, the document asked for first.
struct LoweredSet {
    bool ok = false;
    std::vector<std::filesystem::path> documents;
    nlohmann::json refusal;
};

LoweredSet lowerSet(const std::filesystem::path &fixture, const std::filesystem::path &dir) {
    const char *bin = std::getenv("SCE_CODEGEN_BIN");
    EXPECT_NE(bin, nullptr) << "SCE_CODEGEN_BIN must be set by CMake add_test ENVIRONMENT";
    LoweredSet set;
    if (bin == nullptr) {
        return set;
    }
    const std::string base = std::string("\"") + bin + "\" lower \"" + fixture.string() + "\" --out-dir \"" +
                             dir.string() + "\" --error-format=json";
    const Ran ran = capture(base + " 2>/dev/null");
    if (ran.status == 0) {
        const auto line = nlohmann::json::parse(ran.output, nullptr, false);
        if (line.is_object() && line.value("kind", std::string{}) == "lower") {
            set.ok = true;
            for (const auto &document : line.at("documents")) {
                set.documents.emplace_back(document.at("path").get<std::string>());
            }
        }
        return set;
    }
    const Ran refused = capture(base + " 2>&1 >/dev/null");
    const auto eol = refused.output.find('\n');
    set.refusal = nlohmann::json::parse(refused.output.substr(0, eol), nullptr, false);
    return set;
}

/// Whether a value the script engine holds is the scenario's. Both are read
/// as JSON, so `253` and `253.0`, or `true` and `true`, are the same value.
bool holds(const nlohmann::json &got, const nlohmann::json &want) {
    return got == want;
}

}  // namespace

class AStaticDatamodelRunsLoweredUnderTheInterpreterTest : public ::testing::Test {
protected:
    void SetUp() override {
        engine_ = &ScriptEngineProvider::getScriptEngine();
        engine_->reset();
    }

    void TearDown() override {
        if (engine_) {
            engine_->shutdown();
        }
    }

    /// A variable of the running machine, as the JSON the script engine
    /// writes for it. Undefined (`JSON.stringify` of nothing) reads as null.
    nlohmann::json variable(const StateMachine &machine, const std::string &name) {
        auto result = engine_->evaluateExpression(machine.getSessionId(), "JSON.stringify(" + name + ")").get();
        if (!result.isSuccess()) {
            ADD_FAILURE() << "reading `" << name << "`: " << result.getErrorMessage();
            return nullptr;
        }
        const auto text = result.getValue<std::string>();
        return text.empty() ? nlohmann::json(nullptr) : nlohmann::json::parse(text, nullptr, false);
    }

    /// What a `<send>` and an `<invoke>` need to run: a dispatcher. Without one
    /// the Interpreter raises error.execution for a `<send>`, as for a send with
    /// nowhere to go, and a child session has no way to answer its parent. The
    /// events a machine sends itself are queued, and run when the raiser is
    /// asked to; the raiser is returned for that.
    std::shared_ptr<EventRaiserImpl> wire(StateMachine &machine) {
        auto scheduler = std::make_shared<EventSchedulerImpl>(
            [](const EventDescriptor &event, std::shared_ptr<IEventTarget> target, const std::string &) -> bool {
                try {
                    return target->send(event).get().isSuccess;
                } catch (...) {
                    return false;
                }
            });
        auto eventRaiser = std::make_shared<EventRaiserImpl>();
        eventRaiser->setScheduler(scheduler);
        eventRaiser->setImmediateMode(false);
        machine.setEventRaiser(eventRaiser);
        machine.setEventDispatcher(
            std::make_shared<EventDispatcherImpl>(scheduler, std::make_shared<EventTargetFactoryImpl>(eventRaiser)));
        return eventRaiser;
    }

    /// Whether `done` comes true within a few seconds, with the machine's queue
    /// run on every look. A child session runs on a thread of its own and
    /// answers its parent through the dispatcher, so what its run leaves in the
    /// parent is there when the parent has been given time to be told, not
    /// when `start` returns.
    static bool settles(const std::shared_ptr<EventRaiserImpl> &raiser, const std::function<bool()> &done,
                        std::chrono::milliseconds within = std::chrono::seconds(5)) {
        const auto deadline = std::chrono::steady_clock::now() + within;
        while (std::chrono::steady_clock::now() < deadline) {
            raiser->processQueuedEvents();
            if (done()) {
                return true;
            }
            std::this_thread::sleep_for(std::chrono::milliseconds(10));
        }
        return false;
    }

    /// Replay `scenario` against `document`, the machine the scenario names
    /// once lowered.
    void replay(const std::string &document, const nlohmann::json &scenario) {
        const auto machine = std::make_shared<StateMachine>(*engine_);
        const auto eventRaiser = wire(*machine);
        ASSERT_TRUE(machine->loadSCXMLFromString(document)) << "the Interpreter does not load the lowered document";
        ASSERT_TRUE(machine->start());
        eventRaiser->processQueuedEvents();

        const auto &steps = scenario.at("steps");
        ASSERT_FALSE(steps.empty()) << "a scenario with no steps judges nothing";
        for (std::size_t n = 0; n < steps.size(); ++n) {
            const auto &step = steps[n];
            const std::string note = step.value("note", std::string{});
            SCOPED_TRACE("step " + std::to_string(n) + (note.empty() ? "" : " (" + note + ")"));
            // A step that moves the machine's time on, for a scenario of a delayed
            // send. The Interpreter's scheduler runs on the wall clock, so the
            // time is waited out; the scenario's margins are wide enough for the
            // wait to be the one the step names.
            if (step.contains("advance_ms")) {
                std::this_thread::sleep_for(std::chrono::milliseconds(step.at("advance_ms").get<int64_t>()));
                eventRaiser->processQueuedEvents();
            }
            if (step.contains("event")) {
                const std::string data = step.contains("data") ? step.at("data").dump() : std::string{};
                machine->processEvent(step.at("event").get<std::string>(), data);
                eventRaiser->processQueuedEvents();
            }
            const auto &expect = step.at("expect");
            // A machine that ended in a top-level <final> has no saved state
            // to read on the generated backends — the save refuses one — so
            // what a scenario says of it is that it ended, and that is the
            // whole of the step.
            if (expect.value("ended", false)) {
                EXPECT_FALSE(expect.contains("state") || expect.contains("variables"))
                    << "an ended machine has no state or variables to read";
                EXPECT_TRUE(machine->isInFinalState()) << "the machine ended in a top-level <final>";
                // What its `<donedata>` left for the invoking parent is read as
                // the generated backends read it: the JSON of the done event's
                // data, compared as JSON so that the members' order is no part
                // of the answer. A `<content expr>` that cannot be evaluated has
                // the empty string as its value (§scxml-5.6.2): the generated
                // backends write it as the JSON string `""`, and the
                // Interpreter leaves the event's data as no text at all, which
                // is the same value.
                if (expect.contains("donedata")) {
                    const std::string written = machine->donedataAtFinal();
                    const auto got =
                        written.empty() ? nlohmann::json("") : nlohmann::json::parse(written, nullptr, false);
                    EXPECT_FALSE(got.is_discarded()) << "the donedata is JSON: " << written;
                    EXPECT_EQ(got, expect.at("donedata")) << "the data the final's <donedata> left";
                }
                continue;
            }
            if (expect.contains("state")) {
                EXPECT_EQ(expect.at("state").get<std::string>(), machine->getCurrentState()) << "the current state";
            }
            if (expect.contains("variables")) {
                for (const auto &[name, want] : expect.at("variables").items()) {
                    const auto got = variable(*machine, name);
                    EXPECT_TRUE(holds(got, want))
                        << "variable `" << name << "` is " << got.dump() << ", not " << want.dump();
                }
            }
        }
    }

    IScriptEngine *engine_ = nullptr;
};

// Every scenario the generated backends replay, on the machine it drives,
// lowered and run by the Interpreter.
TEST_F(AStaticDatamodelRunsLoweredUnderTheInterpreterTest, TheInterpreterDoesWhatTheGeneratedBackendsDo) {
    std::vector<std::string> replayed;
    std::vector<std::string> notYetLowered;
    for (const auto &entry : std::filesystem::directory_iterator(kFixtures / "scenarios")) {
        if (entry.path().extension() != ".json") {
            continue;
        }
        const std::string name = entry.path().stem().string();
        SCOPED_TRACE(name);
        const auto scenario = nlohmann::json::parse(slurp(entry.path()), nullptr, false);
        ASSERT_FALSE(scenario.is_discarded()) << "a scenario is JSON";
        const std::string machine = scenario.at("machine").get<std::string>();

        const Lowered lowered = lower(kFixtures / (machine + ".scxml"));
        if (!lowered.ok) {
            // A refusal is an answer, and it must be the typed one: a crash
            // or a bare failure would read the same as "not yet lowered".
            ASSERT_TRUE(lowered.refusal.is_object()) << "`sce-codegen lower` failed with no diagnostic";
            EXPECT_EQ("generate/unsupported-feature", lowered.refusal.value("code", std::string{}))
                << lowered.refusal.dump();
            notYetLowered.push_back(name);
            continue;
        }
        replay(lowered.document, scenario);
        replayed.push_back(name);
    }

    // Floor: a scan that found nothing to replay would pass, so what it must
    // have replayed is asserted. Every scenario is of a machine whose every
    // construct the lowering covers today, and the floor rises as the lowering
    // grows.
    EXPECT_GE(replayed.size(), 14u) << "replayed " << replayed.size()
                                    << " scenarios, not yet lowered: " << notYetLowered.size();
    // A byte string is the text of its bytes in the Interpreter's data model, one
    // character to a byte (docs/adr/0005, decision 2), so the scenario that holds one
    // is replayed and not left to a refusal the scan above would let pass.
    EXPECT_NE(std::find(replayed.begin(), replayed.end(), "static_bytes"), replayed.end())
        << "static_bytes was not replayed: a byte string has no lowering for the Interpreter";
    // ... and so is a record's field of one, the object that holds it written again with the
    // field changed.
    EXPECT_NE(std::find(replayed.begin(), replayed.end(), "static_record_bytes"), replayed.end())
        << "static_record_bytes was not replayed: a record's byte string has no lowering for the Interpreter";
    // ... and the bytes a typed payload carries, the text its wire spells them as.
    EXPECT_NE(std::find(replayed.begin(), replayed.end(), "static_payload_bytes"), replayed.end())
        << "static_payload_bytes was not replayed: a payload's byte string has no lowering for the Interpreter";
    // ... and one carried by a `<param>` and a `<donedata>`, as its Latin-1 text.
    EXPECT_NE(std::find(replayed.begin(), replayed.end(), "static_bytes_wire"), replayed.end())
        << "static_bytes_wire was not replayed: a byte string as a param has no lowering for the Interpreter";
    for (const auto &name : notYetLowered) {
        RecordProperty("not_yet_lowered_" + name, "refused by sce-codegen lower");
    }
}

namespace {

/// A host that performs every operation and records each call as the JSON of
/// its arguments.
class RecordingHost : public INativeActionHost {
public:
    bool performNativeAction(const std::string &name, const std::vector<ScriptValue> &args) override {
        nlohmann::json values = nlohmann::json::array();
        for (const auto &arg : args) {
            if (const auto *flag = std::get_if<bool>(&arg)) {
                values.push_back(*flag);
            } else if (const auto *whole = std::get_if<int64_t>(&arg)) {
                values.push_back(*whole);
            } else if (const auto *real = std::get_if<double>(&arg)) {
                values.push_back(*real);
            } else if (const auto *text = std::get_if<std::string>(&arg)) {
                values.push_back(*text);
            } else {
                values.push_back(nullptr);
            }
        }
        calls.push_back({name, values});
        return true;
    }

    std::vector<std::pair<std::string, nlohmann::json>> calls;
};

}  // namespace

// `static_host_call` is the one fixture with no scenario: its host operation is
// the host's, so what it holds is the calls the host was given. The generated
// backends hold the same calls in their own tests (a generated recording host
// on Rust and Kotlin); here the Interpreter performs the action through the host
// installed on the machine, with the arguments its engine computes from the
// lowered expressions — a variable, and a comparison over it.
TEST_F(AStaticDatamodelRunsLoweredUnderTheInterpreterTest, AHostActionIsPerformedWithTheValuesItsArgumentsComputeTo) {
    const Lowered lowered = lower(kFixtures / "static_host_call.scxml");
    ASSERT_TRUE(lowered.ok) << "`<sce:action>` is lowered for the Interpreter now: " << lowered.refusal.dump();

    const auto host = std::make_shared<RecordingHost>();
    const auto machine = std::make_shared<StateMachine>(*engine_);
    machine->setNativeActionHost(host);
    ASSERT_TRUE(machine->loadSCXMLFromString(lowered.document));
    ASSERT_TRUE(machine->start());
    for (int retry = 0; retry < 4; ++retry) {
        machine->processEvent("retry", "");
    }

    ASSERT_EQ(host->calls.size(), 4u) << "one call per entry of `idle`; the fourth retry finds `attempts < 3` false";
    const std::vector<std::pair<int, bool>> expected = {{0, false}, {1, false}, {2, false}, {3, true}};
    for (std::size_t n = 0; n < expected.size(); ++n) {
        SCOPED_TRACE("call " + std::to_string(n));
        EXPECT_EQ(host->calls[n].first, "showAttempts");
        EXPECT_EQ(host->calls[n].second, nlohmann::json::array({expected[n].first, expected[n].second}))
            << "each with the datamodel as it stood";
    }
}

// A document under another data model is not the lowering's to touch: what it
// prints for one is the document.
TEST_F(AStaticDatamodelRunsLoweredUnderTheInterpreterTest, ADocumentUnderAnotherDataModelIsPrintedAsItIs) {
    const auto dir = std::filesystem::temp_directory_path() / ("sce-lower-ecmascript-" + std::to_string(::getpid()));
    std::filesystem::create_directories(dir);
    const auto path = dir / "plain.scxml";
    const std::string document =
        R"(<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="ecmascript" initial="s">
  <datamodel><data id="level" expr="250"/></datamodel>
  <state id="s"/>
</scxml>)";
    {
        std::ofstream out(path);
        out << document;
    }
    const Lowered lowered = lower(path);
    std::error_code ec;
    std::filesystem::remove_all(dir, ec);
    ASSERT_TRUE(lowered.ok);
    EXPECT_EQ(document, lowered.document);
}

// An `<invoke type="scxml">` hands its child the values its `<param>`s and
// `namelist` name, each to the child's variable of the same name, and a variable
// nothing hands a value to keeps the one its `<data>` gave it
// (`static_invoke_params.scxml`; the generated backends' halves are
// `a_static_child_is_handed_its_params`). The child is a `sce-static` document of
// its own, lowered where it stands: `worker` ends only on `start === 7 && enabled`.
TEST_F(AStaticDatamodelRunsLoweredUnderTheInterpreterTest, AChildIsHandedTheValuesItsInvokeNames) {
    const Lowered lowered = lower(kFixtures / "static_invoke_params.scxml");
    ASSERT_TRUE(lowered.ok) << lowered.refusal.dump();
    const auto machine = std::make_shared<StateMachine>(*engine_);
    const auto raiser = wire(*machine);
    ASSERT_TRUE(machine->loadSCXMLFromString(lowered.document)) << "the Interpreter does not load the document";
    ASSERT_TRUE(machine->start());

    // `worker` was handed 7 (the value `base` holds when the invoke executes, after
    // the entry action added 3) and true, so it ended: the parent left `working`
    // for `plain` and counted it.
    ASSERT_TRUE(settles(raiser, [&] { return variable(*machine, "completed") == 1; }))
        << "the child was not handed `start` and `enabled`, so it never ended";
    EXPECT_EQ("plain", machine->getCurrentState());

    // `control`, the same child handed nothing, keeps its declared defaults and
    // never ends (`done.invoke.control` would add 100); `watcher` was handed 7 and
    // ends only at 8, and `bump` raising `base` does not reach it: a child is
    // handed its values once, when it starts.
    machine->processEvent("bump", "");
    EXPECT_FALSE(settles(
        raiser, [&] { return variable(*machine, "completed") != 1; }, std::chrono::milliseconds(300)))
        << "a child that was handed nothing, or handed a value again, ended";
}

// A string an `<invoke>` hands its child is held to the bound the child declared
// for the variable, in UTF-8 bytes: a value past it is left out, with an
// error.execution, and the child starts holding the one its `<data>` gave it
// (`static_invoke_string.scxml`). The bound is carried by the lowered `<param>`.
TEST_F(AStaticDatamodelRunsLoweredUnderTheInterpreterTest, AStringHandedToAChildIsHeldToItsBound) {
    const Lowered lowered = lower(kFixtures / "static_invoke_string.scxml");
    ASSERT_TRUE(lowered.ok) << lowered.refusal.dump();
    const auto machine = std::make_shared<StateMachine>(*engine_);
    const auto raiser = wire(*machine);
    ASSERT_TRUE(machine->loadSCXMLFromString(lowered.document)) << "the Interpreter does not load the document";
    ASSERT_TRUE(machine->start());

    // `fits` ends on 'wxyz' (1), `over` was handed 8 bytes past its 4 and ends on the
    // 'ab' its `<data>` gave it (10), `wide` was handed five bytes in two characters
    // and ends on the same (100); the two left out raise an error each.
    EXPECT_TRUE(
        settles(raiser, [&] { return variable(*machine, "completed") == 111 && variable(*machine, "errors") == 2; }))
        << "completed is " << variable(*machine, "completed").dump() << " and errors "
        << variable(*machine, "errors").dump();
}

// A child takes the events its parent forwards, and its end is one
// `done.invoke.<id>` the parent counts (`static_invoke.scxml`).
TEST_F(AStaticDatamodelRunsLoweredUnderTheInterpreterTest, AChildTakesTheEventsItsParentForwards) {
    const Lowered lowered = lower(kFixtures / "static_invoke.scxml");
    ASSERT_TRUE(lowered.ok) << lowered.refusal.dump();
    const auto machine = std::make_shared<StateMachine>(*engine_);
    const auto raiser = wire(*machine);
    ASSERT_TRUE(machine->loadSCXMLFromString(lowered.document)) << "the Interpreter does not load the document";
    ASSERT_TRUE(machine->start());
    raiser->processQueuedEvents();
    EXPECT_EQ("working", machine->getCurrentState());

    // `a` moves the child on, and it has not ended.
    machine->processEvent("a", "");
    EXPECT_FALSE(settles(
        raiser, [&] { return variable(*machine, "completed") != 0; }, std::chrono::milliseconds(300)))
        << "the child ended on `a`, which only moves it on";

    // `b` ends it: the parent counts `done.invoke.worker` and stays where it is.
    machine->processEvent("b", "");
    ASSERT_TRUE(settles(raiser, [&] { return variable(*machine, "completed") == 1; }))
        << "the child ended on `b` and its end was never counted";
    EXPECT_EQ("working", machine->getCurrentState());

    // `abort` leaves `working`, which cancels a child, and `idle` invokes nothing.
    machine->processEvent("abort", "");
    raiser->processQueuedEvents();
    EXPECT_EQ("idle", machine->getCurrentState());
}

// A hybrid `<invoke>` starts the candidate its `srcexpr` value names, by the stem of
// the document it names (`static_invoke_hybrid.scxml`; the generated backends' halves
// are `a_static_hybrid_invoke_starts_the_candidate_its_value_names`). The Interpreter
// loads the document at run time from beside the one that invokes it, so the lowering
// is a directory: the invoking document and each candidate, lowered. Four phases —
// a `file:` value, an absolute path, an argument no 32-bit field can hold, and a
// document the invoke did not declare — and each `done.invoke` adds a power of ten, so
// the sum 111 and the two errors say which candidates ended and what was reported.
TEST_F(AStaticDatamodelRunsLoweredUnderTheInterpreterTest, AHybridInvokeStartsTheCandidateItsValueNames) {
    const auto dir = std::filesystem::temp_directory_path() / ("sce-lower-hybrid-" + std::to_string(::getpid()));
    std::error_code ec;
    std::filesystem::remove_all(dir, ec);
    const LoweredSet set = lowerSet(kFixtures / "static_invoke_hybrid.scxml", dir);
    ASSERT_TRUE(set.ok) << "a hybrid `<invoke>` is lowered for the Interpreter now: " << set.refusal.dump();
    ASSERT_EQ(3u, set.documents.size()) << "the invoking document and its two candidates";
    EXPECT_EQ(dir / "static_invoke_hybrid.scxml", set.documents[0]);

    const auto machine = std::make_shared<StateMachine>(*engine_);
    const auto raiser = wire(*machine);
    ASSERT_TRUE(machine->loadSCXML(set.documents[0].string())) << "the Interpreter does not load the document";
    ASSERT_TRUE(machine->start());

    // The candidate each value names ends on the two values it is waiting for. One
    // handed the other's names would never end, and the run would stop short.
    EXPECT_TRUE(
        settles(raiser, [&] { return variable(*machine, "completed") == 111 && variable(*machine, "errors") == 2; }))
        << "completed is " << variable(*machine, "completed").dump() << " and errors "
        << variable(*machine, "errors").dump();
    EXPECT_TRUE(machine->isInFinalState()) << "the run ended in `over`";
    std::filesystem::remove_all(dir, ec);
}

// The library's `candidate` reduces the value to a document stem by the rule every
// engine reads from the one table, `tests/document_stem/document_stem.json`: the
// build's reader, the six runtimes' and this one cannot disagree about the stem of
// one document. A value that names a declared stem answers the file name the lowered
// candidate is written under; one that names none throws, which the Interpreter
// reports as an `srcexpr` that could not be evaluated.
TEST_F(AStaticDatamodelRunsLoweredUnderTheInterpreterTest, TheLibraryReducesAValueToTheStemEveryEngineReducesItTo) {
    const char *bin = std::getenv("SCE_CODEGEN_BIN");
    ASSERT_NE(bin, nullptr) << "SCE_CODEGEN_BIN must be set by CMake add_test ENVIRONMENT";
    // Any document that is lowered installs the library; an algorithm is the
    // smallest, and `lower-algorithm` hands over the expression that installs it.
    const Ran ran =
        capture(std::string("\"") + bin + "\" lower-algorithm sce:std/time/second_of_day.scxml 2>/dev/null");
    ASSERT_EQ(0, ran.status);
    const auto line = nlohmann::json::parse(ran.output, nullptr, false);
    ASSERT_TRUE(line.is_object()) << ran.output.substr(0, 200);

    const std::string session = "document_stem_lowered";
    ASSERT_TRUE(engine_->createSession(session, ""));
    const auto installed = engine_->executeScript(session, line.at("install").get<std::string>()).get();
    ASSERT_TRUE(installed.isSuccess()) << installed.getErrorMessage();

    const auto table = nlohmann::json::parse(
        slurp(std::filesystem::path(SCE_PROJECT_ROOT) / "tests/document_stem/document_stem.json"), nullptr, false);
    ASSERT_FALSE(table.is_discarded()) << "the table is JSON";
    ASSERT_GE(table.at("cases").size(), 15u) << "the table lost cases";
    for (const auto &entry : table.at("cases")) {
        const std::string value = entry.at("value").get<std::string>();
        const std::string stem = entry.at("stem").get<std::string>();
        SCOPED_TRACE(entry.at("name").get<std::string>() + ": " + value);
        // JSON text of a string is a string literal of the script, escapes included.
        const std::string literal = nlohmann::json(value).dump();
        if (!stem.empty()) {
            const auto named = engine_
                                   ->evaluateExpression(session, "SceStatic.candidate(" + literal + ", [" +
                                                                     nlohmann::json(stem).dump() + "])")
                                   .get();
            ASSERT_TRUE(named.isSuccess()) << named.getErrorMessage();
            EXPECT_EQ(stem + ".scxml", named.getValue<std::string>());
        }
        // A list that does not hold the stem names no document.
        const auto other = engine_
                               ->evaluateExpression(session, "SceStatic.candidate(" + literal + ", [" +
                                                                 nlohmann::json(stem + "-other").dump() + "])")
                               .get();
        EXPECT_FALSE(other.isSuccess()) << "a value naming a document the invoke does not declare";
    }
    engine_->destroySession(session);
}

}  // namespace Tests
}  // namespace SCE
