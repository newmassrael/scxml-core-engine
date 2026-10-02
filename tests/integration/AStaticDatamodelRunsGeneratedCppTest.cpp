// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// datamodel="sce-static" (docs/SCE_ACCEPTED_SUBSET.md §2.15) under generated
// C++: a variable is a member of the policy, every expression was lowered to C++
// at build time, and no script engine is built.
//
// The scenarios here are the ones Kotlin, Rust and the Interpreter replay —
// `sce-build/tests/fixtures/static_datamodel/scenarios/<machine>.json`, whose
// expected values are derived from the document, not observed from a backend —
// so the four engines are held to one answer. The machines are generated from
// the fixtures beside them by `tests/CMakeLists.txt`.
//
// What a failing scenario would say:
//
//   * `static_counter`: guards, `<assign>`, `<if>`/`<elseif>` and `In()` are
//     native, and `count` reaches the host as the `uint32` it is.
//   * `static_overflow`: a checked integer operation that overflows is a
//     failure, not a wrapped value (§3.4.1) — the variable keeps what it held,
//     `error.execution` is raised, and a guard over the overflowing sum is false.
//   * `static_block_ends`: an error ends the block it stands in (W3C SCXML 4.9)
//     — the statements after a failed `<assign>`, after a failed `<if>` cond,
//     or inside a branch that failed do not run, while the next block does.

#include "static_block_ends_sm.h"
#include "static_counter_sm.h"
#include "static_overflow_sm.h"

#include <filesystem>
#include <fstream>
#include <functional>
#include <gtest/gtest.h>
#include <map>
#include <nlohmann/json.hpp>
#include <sstream>
#include <string>

#ifndef SCE_PROJECT_ROOT
#define SCE_PROJECT_ROOT "."
#endif

namespace SCE {
namespace Tests {

namespace {

using json = nlohmann::json;

const std::string kStaticFixtures = std::string(SCE_PROJECT_ROOT) + "/sce-build/tests/fixtures/static_datamodel";

json readScenario(const std::string &machine) {
    const auto path = std::filesystem::path(kStaticFixtures) / "scenarios" / (machine + ".json");
    std::ifstream in(path);
    EXPECT_TRUE(in.is_open()) << "not readable: " << path;
    return json::parse(in);
}

/// One generated machine as a scenario sees it: events by their document name,
/// and the published variables by theirs.
template <typename Machine> class Driver {
public:
    using Reader = std::function<json(const Machine &)>;

    explicit Driver(std::map<std::string, Reader> variables) : variables_(std::move(variables)) {}

    void start() {
        machine_.initialize();
    }

    /// An event goes in, and the macrostep it starts runs to the end.
    void send(const std::string &name) {
        machine_.raiseExternal(name);
        machine_.step();
    }

    std::string state() const {
        return Machine::PolicyType::getStateName(machine_.getCurrentState());
    }

    bool ended() const {
        return machine_.isInFinalState();
    }

    json variable(const std::string &name) const {
        const auto found = variables_.find(name);
        EXPECT_NE(found, variables_.end()) << "the driver publishes no variable '" << name << "'";
        return found == variables_.end() ? json() : found->second(machine_);
    }

private:
    Machine machine_;
    std::map<std::string, Reader> variables_;
};

/// Replay `machine`'s scenario file against `driver`. Every step names an event
/// (or none, for the machine as started) and what it must hold afterwards.
template <typename Machine> void replay(const std::string &machine, Driver<Machine> &driver) {
    const json scenario = readScenario(machine);
    ASSERT_TRUE(scenario.contains("steps")) << machine;
    driver.start();
    int index = 0;
    for (const auto &step : scenario["steps"]) {
        SCOPED_TRACE(machine + " step " + std::to_string(index++) + ": " + step.value("note", std::string{}));
        if (step.contains("event")) {
            driver.send(step["event"].get<std::string>());
        }
        const auto &expect = step["expect"];
        if (expect.value("ended", false)) {
            EXPECT_TRUE(driver.ended());
            continue;
        }
        if (expect.contains("state")) {
            EXPECT_EQ(driver.state(), expect["state"].get<std::string>());
        }
        if (expect.contains("variables")) {
            for (const auto &[name, value] : expect["variables"].items()) {
                EXPECT_EQ(driver.variable(name), value) << "variable '" << name << "'";
            }
        }
    }
}

}  // namespace

namespace G = SCE::Generated;

/// The counter's variables as a host reads them. One machine, two scenarios.
Driver<G::static_counter::static_counter> counterDriver() {
    using Machine = G::static_counter::static_counter;
    return Driver<Machine>({
        {"count", [](const Machine &m) { return json(m.count()); }},
        {"ready", [](const Machine &m) { return json(m.ready()); }},
    });
}

TEST(AStaticDatamodelRunsGeneratedCppTest, TheCounterCountsToItsFlagAndLetsGo) {
    auto driver = counterDriver();
    replay("static_counter", driver);
}

TEST(AStaticDatamodelRunsGeneratedCppTest, TheCounterStopsAtItsBoundAndRefusesGo) {
    auto driver = counterDriver();
    replay("static_counter_bound", driver);
}

TEST(AStaticDatamodelRunsGeneratedCppTest, AnOverflowingOperationFailsInsteadOfWrapping) {
    using Machine = G::static_overflow::static_overflow;
    Driver<Machine> driver({
        {"level", [](const Machine &m) { return json(m.level()); }},
        {"refusals", [](const Machine &m) { return json(m.refusals()); }},
    });
    replay("static_overflow", driver);
}

TEST(AStaticDatamodelRunsGeneratedCppTest, AnErrorEndsTheBlockItStandsIn) {
    using Machine = G::static_block_ends::static_block_ends;
    Driver<Machine> driver({
        {"a", [](const Machine &m) { return json(m.a()); }},
        {"b", [](const Machine &m) { return json(m.b()); }},
        {"afterAssign", [](const Machine &m) { return json(m.afterAssign()); }},
        {"thenRan", [](const Machine &m) { return json(m.thenRan()); }},
        {"elseRan", [](const Machine &m) { return json(m.elseRan()); }},
        {"afterIf", [](const Machine &m) { return json(m.afterIf()); }},
        {"inBranch", [](const Machine &m) { return json(m.inBranch()); }},
        {"afterBranch", [](const Machine &m) { return json(m.afterBranch()); }},
        {"afterOk", [](const Machine &m) { return json(m.afterOk()); }},
        {"errors", [](const Machine &m) { return json(m.errors()); }},
    });
    replay("static_block_ends", driver);
}

}  // namespace Tests
}  // namespace SCE
