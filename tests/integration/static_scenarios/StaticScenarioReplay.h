// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Replay of an engine-neutral scenario against a generated C++ machine
// (`datamodel="sce-static"`, docs/SCE_ACCEPTED_SUBSET.md §2.15). The C++ half of the
// Rust replay in `backends/rust/scenarios/src/lib.rs`, and the same design.
//
// A scenario (`sce-build/tests/fixtures/static_datamodel/scenarios/*.json`) is data: a
// list of steps, each an external event with its payload and what the machine must hold
// after it runs to quiescence — its current state, its active states, any of its
// variables, what it asked of its host, what each child session was given as its host
// and asked of it, and what it sent its Mesh peers. What the machine holds is read from
// its saved state, the text every backend saves byte for byte (docs/adr/0045), so one
// scenario judges every backend by the same answer and a machine needs no reader of its
// own here. The replay used to read each variable through a reader written per machine,
// with a converter per record type; every one of those had to be compiled where the
// replay was, and the 76 machines they reached made one translation unit that took
// 20.57 GB to compile (measured 2026-10-23), more than the hosted runner holds.
//
// Which machine a scenario replays is the scenario's own `machine`, and the build makes
// one translation unit per machine from the scenario directory
// (`StaticScenarioMachineTest.cpp.in`, `sce_add_static_scenario_suite` in
// tests/CMakeLists.txt): a scenario added to the directory is replayed with no edit
// here, and no unit holds more than one machine.
//
// A machine whose host takes part in its run — one with `<sce:action>`s, one that states
// a duty, one whose children are given hosts — is built through a specialization of
// `ScenarioHosts` for its type, in `hosts/<machine>.h` beside this file, which the
// machine's unit includes when it exists. Every other machine is built with no host.

#pragma once

#include <cstdint>
#include <deque>
#include <memory>
#include <optional>
#include <set>
#include <string>
#include <type_traits>
#include <utility>

#include <gtest/gtest.h>
#include <nlohmann/json.hpp>

#include "common/SceClock.h"
#include "core/StatePolicyConcepts.h"
#include "saved/SavedState.h"
#include "static/StaticExecutionEngine.h"

namespace SCE::Tests::StaticScenario {

using json = nlohmann::json;

/// The scenario of that name, read from its file.
json readScenario(const std::string &scenario);

/// Whether a saved value is the scenario's. A saved state writes a 64-bit integer as its
/// decimal text (a JSON number past 2^53 is not exact everywhere), so an expected integer
/// matches that text of the same integer; everything else must be equal as written.
bool holds(const json &saved, const json &expected);

/// The child hosts a machine's host was asked for, in the order asked, and what each child
/// asked of its host. A child owns nothing of the host it was given, the machine's host
/// does, so each host is opened here and writes into the book this reads. The children are
/// kept in a deque, whose elements stay put, because a host holds a reference to its own.
class ChildHostBook {
public:
    class Child {
    public:
        Child(std::string invoke, int start, std::string candidate);

        /// The child asked its host for `action`, with `args` in the order the document
        /// gives them.
        void call(const std::string &action, json args = json::array());

    private:
        friend class ChildHostBook;
        std::string invoke_;
        int start_;
        std::string candidate_;
        json calls_ = json::array();
        std::size_t taken_ = 0;
        bool announced_ = false;
    };

    /// The machine's host was asked for the host of a child its `invoke` started:
    /// `candidate` is the stem of the document when the invoke names one of several, and
    /// empty for a child written inline.
    Child &open(const std::string &invoke, const std::string &candidate = "");

    /// The children asked for or called since this was last called, oldest asked first,
    /// each as a scenario's `child_hosts` writes it: the invoke, the start of that invoke,
    /// the candidate's document stem where the invoke names one, and the calls made in
    /// this step alone. A child asked for earlier that called nothing since has no entry.
    json take();

private:
    std::deque<Child> children_;
};

/// The routers a replay gives the child sessions of the machine it drives (SCE_MESH.md,
/// Child Sessions in Mesh; docs/adr/0043). A child that sends to a Mesh peer is given a
/// router of its own by its parent, which asks the factory its host registered; a replay
/// is the host, so it registers one (`factory()`) whose routers write down what each child
/// sent, told by the path of invoke ids that started it and how many children that path
/// had started, and hold what a step's `invoke` delivers to the child until its parent
/// applies it. A router answers for the child's own children in turn, so a grandchild is
/// told by `<invoke>/<invoke>`. The C++ half of the Rust replay's `child_mesh::Children`.
class ChildRouters {
public:
    /// Children whose sends are written into `sent`, beside the machine's own.
    explicit ChildRouters(std::shared_ptr<json> sent);

    /// The factory a replay registers on the machine: every child is given a router,
    /// whichever invoke starts it and from whatever document.
    ::SCE::Static::ChildMeshRouters factory() const;

    /// Hand `event`, with `data` as its `_event.data`, to the running child of `path`, as
    /// its parent would be handed it: the parent applies it the next time it drives the
    /// child. False when no child of `path` is running, which a step that names one fails
    /// on.
    bool deliver(const std::string &path, const std::string &event, const std::string &data) const;

private:
    struct Book;
    class Recording;
    static std::shared_ptr<::SCE::Static::IChildMeshRouter> start(const std::shared_ptr<Book> &book,
                                                                  const std::string &path);

    std::shared_ptr<Book> book_;
};

/// How the replay builds `Machine` and reads what its hosts did. The primary template is
/// the machine with no host: it is built with no argument, and nothing records host calls
/// or child hosts, so a step that states either fails. A machine whose host takes part in
/// its run specializes this in `hosts/<machine>.h`, with the same members:
///
///   std::unique_ptr<Machine> make();   // called again by a step that restores
///   std::optional<json> hostCalls();   // what the host was asked since the last call
///   std::optional<json> childHosts();  // the child hosts asked for since the last call
///
/// `std::nullopt` from either is a host that does not record that.
template <typename Machine> struct ScenarioHosts {
    std::unique_ptr<Machine> make() {
        return std::make_unique<Machine>();
    }

    std::optional<json> hostCalls() {
        return std::nullopt;
    }

    std::optional<json> childHosts() {
        return std::nullopt;
    }
};

/// How many rounds a host gives a machine that needs `tick` after an event before it
/// reads the machine back: two for each level of child session, and a round that finds
/// nothing to do changes nothing.
inline constexpr int kSettleTicks = 5;

/// The id a shared fixture's root session is given, and the moment on the wall clock a
/// scenario's `restore` step and every reading save the machine at (docs/adr/0045,
/// section 8, and "Questions the owner answered" 2).
inline constexpr const char *kRootSessionId = "fixture-root";
inline constexpr std::int64_t kSavedAtMs = 1'700'000'000'000;

/// The rounds `machine` needs before it is read back. The generated policy says which
/// call it needs: `step` drains the queues and nothing else, and a machine with a delayed
/// send or a child session is driven by `tick`, which also runs those. The engine reports
/// no point at which it has settled, so a host runs rounds: a child takes an event the
/// parent forwarded in one round and its end reaches the parent in the next.
template <typename Machine> void settleRounds(Machine &machine) {
    if constexpr (::SCE::Core::NeedsEventScheduler<typename Machine::PolicyType>) {
        for (int round = 0; round < kSettleTicks; ++round) {
            machine.tick();
        }
    } else {
        machine.step();
    }
}

/// Whether a generated machine can be saved. A machine a saved state cannot hold whole is
/// generated without the save API (`saved_shape` in sce-build/src/forge/static_lowering.rs),
/// and such a machine has no scenario: the replay reads every machine through its save.
template <typename M, typename = void> struct CanBeSaved : std::false_type {};

template <typename M>
struct CanBeSaved<M, std::void_t<decltype(std::declval<M &>().saveAt(std::int64_t{0}))>> : std::true_type {};

/// One generated machine as a scenario drives it, on a manual clock, with the host
/// `ScenarioHosts<Machine>` gives it and a recorder of what it sends its Mesh peers.
template <typename Machine> class Replay {
    static_assert(CanBeSaved<Machine>::value,
                  "a scenario reads its machine through the machine's saved state, and this machine is generated "
                  "without one: a saved state cannot hold it whole");

public:
    /// What the machine's host registers on it, before it starts: the callback its sends
    /// to a Mesh peer are handed to, and the factory of its children's routers. A scenario
    /// that says `"router": false` is of a machine whose host attached no link
    /// (docs/adr/0046, section 4): it is given neither, and every send to a peer is
    /// refused where it is made.
    ///
    /// The machine runs on a manual clock, so a delayed send waits exactly the time a step
    /// names (`advance_ms`) and not the time the test happened to take.
    explicit Replay(bool routed) : machine_(hosts_.make()), routed_(routed) {
        prepare();
        machine_->initialize();
        // A machine that starts a child session in its first state has that child
        // running, and maybe ended, before any event arrives; it is read as the events
        // leave it, after the rounds that take.
        if constexpr (::SCE::Core::NeedsEventScheduler<typename Machine::PolicyType>) {
            settleRounds(*machine_);
        }
    }

    /// Save the machine, discard it and restore it from the text, as a state that crossed
    /// a process boundary would be: the restored machine carries on where the saved one
    /// stood (docs/adr/0045, section 8). It is built as the first was and given what the
    /// host gave the first, and the children it resumed are asked for their routers when
    /// the factory arrives.
    void restore() {
        const ::SCE::Saved::SavedState saved =
            ::SCE::Saved::SavedState::fromJson(machine_->saveAt(kSavedAtMs).toJson());
        machine_ = hosts_.make();
        machine_->setClock(std::make_shared<::SCE::ManualClock>(0));
        machine_->restoreAt(saved, kSavedAtMs);
        attachLinks();
    }

    /// The machine's time moves on by `ms`, and what that made due runs.
    void advance(std::uint64_t ms) {
        machine_->advanceTimeMs(ms);
    }

    /// An event goes in, and the macrostep it starts runs to the end. `data` is the
    /// event's payload as JSON text, from which the machine lifts the typed fields its
    /// schema names.
    void send(const std::string &name, const std::string &data) {
        machine_->raiseExternal(name, data);
        settleRounds(*machine_);
    }

    /// A peer's event for the running child of `path` (invoke ids joined by `/`), handed
    /// to the router the child was given; the machine applies it when it next drives the
    /// child. False, and nothing run, when no child of `path` is running.
    bool sendToChild(const std::string &path, const std::string &name, const std::string &data) {
        if (!children_.deliver(path, name, data)) {
            return false;
        }
        settleRounds(*machine_);
        return true;
    }

    /// Whether an event arriving under `name` reaches the machine at all, as the engine
    /// decides it (§scxml-3.12.1): false is a name it drops.
    bool resolves(const std::string &name) const {
        return machine_->resolveEventByName(name).has_value();
    }

    bool ended() const {
        return machine_->isInFinalState();
    }

    /// The data the top-level final's `<donedata>` left for the invoking parent, as the
    /// JSON text the done event carries.
    std::string donedata() const {
        return machine_->donedataAtFinal();
    }

    /// The machine as its saved state holds it: `current`, `configuration` and
    /// `variables` among the rest. A machine that ended in a top-level `<final>` has none.
    json saved() const {
        return json::parse(machine_->saveAt(kSavedAtMs).toJson());
    }

    std::optional<json> hostCalls() {
        return hosts_.hostCalls();
    }

    std::optional<json> childHosts() {
        return hosts_.childHosts();
    }

    /// What the machine and its children sent their Mesh peers since this was last
    /// called, oldest first, each as a scenario's `sent` writes it.
    json sent() {
        return std::exchange(*sent_, json::array());
    }

private:
    /// The clock, the root session's id — the shared fixtures give it explicitly, so a
    /// saved text of one is the text on every backend — and the host's links.
    void prepare() {
        machine_->setClock(std::make_shared<::SCE::ManualClock>(0));
        machine_->bindSessionId(kRootSessionId);
        attachLinks();
    }

    /// The callback a send to a Mesh peer is handed to, which writes the send down and
    /// accepts it — the peer is not there, and what a scenario holds a step to is that the
    /// machine asked for the send — and the factory of its children's routers, which
    /// write into the same list.
    void attachLinks() {
        if (!routed_) {
            return;
        }
        machine_->setMeshSendCallback([sent = sent_](const std::string &target, const std::string &event,
                                                     const std::string &data, const std::string &,
                                                     const std::string &) {
            json entry = {{"target", target}, {"event", event}};
            if (!data.empty()) {
                entry["data"] = json::parse(data);
            }
            sent->push_back(std::move(entry));
            return true;
        });
        machine_->registerChildMeshRouters(children_.factory());
    }

    ScenarioHosts<Machine> hosts_;
    std::unique_ptr<Machine> machine_;
    bool routed_;
    /// Shared with the callback the machine holds and with the children's routers.
    std::shared_ptr<json> sent_ = std::make_shared<json>(json::array());
    ChildRouters children_{sent_};
};

/// Replay the scenario of that name against `Machine`. Every step names an event (or
/// none, for the machine as started) and what it must hold afterwards. A name no event of
/// the machine matches is a misspelt step unless the step says it expects the drop
/// (`"dropped": true`), which is then what is held.
template <typename Machine> void replay(const std::string &name) {
    const json scenario = readScenario(name);
    ASSERT_TRUE(scenario.contains("steps")) << name;
    ASSERT_FALSE(scenario["steps"].empty()) << name << ": a scenario with no steps judges nothing";
    Replay<Machine> machine(scenario.value("router", true));
    int index = 0;
    for (const auto &step : scenario["steps"]) {
        SCOPED_TRACE(name + " step " + std::to_string(index++) + ": " + step.value("note", std::string{}));
        // The machine is saved and restored before the step's event or time, so what the
        // step then expects is what the restored one does. What a host recorded is that
        // host's own, and a restored machine is built with a host that records from
        // nothing, so a machine read through its hosts is not restored here.
        if (step.value("restore", false)) {
            ASSERT_FALSE(machine.hostCalls().has_value() || machine.childHosts().has_value())
                << "a machine whose hosts' records a step reads is not restored by this replay";
            machine.restore();
        }
        if (step.contains("advance_ms")) {
            machine.advance(step["advance_ms"].get<std::uint64_t>());
        }
        if (step.contains("event")) {
            const auto event = step["event"].get<std::string>();
            const std::string data = step.contains("data") ? step["data"].dump() : "";
            if (step.contains("invoke")) {
                // A peer's event for the running child of that invoke. The child's events
                // are not the machine's, so the machine is not asked whether it matches.
                const auto path = step["invoke"].get<std::string>();
                EXPECT_TRUE(machine.sendToChild(path, event, data))
                    << "no child of '" << path << "' is running to be sent '" << event << "'";
            } else {
                const bool dropped = step.value("dropped", false);
                EXPECT_EQ(machine.resolves(event), !dropped)
                    << "the machine's events " << (dropped ? "match" : "do not match") << " '" << event << "'";
                machine.send(event, data);
            }
        }
        const auto &expect = step["expect"];
        // What the machine asked of its host in this step, and the child hosts its host
        // was asked for. Taken on every step, so that a step which states nothing does not
        // hand them to the next.
        const std::optional<json> asked = machine.hostCalls();
        if (expect.contains("host_calls")) {
            EXPECT_TRUE(asked.has_value()) << "this machine's host records no calls, so no step can be held to them";
            EXPECT_EQ(asked.value_or(json()), expect["host_calls"]) << "what the machine asked of its host";
        }
        const std::optional<json> children = machine.childHosts();
        if (expect.contains("child_hosts")) {
            EXPECT_TRUE(children.has_value())
                << "this machine's children are given no hosts that record, so no step can be held to them";
            EXPECT_EQ(children.value_or(json()), expect["child_hosts"])
                << "the child hosts the machine's host was asked for, and what their children asked of them";
        }
        // Compared as values, in the order sent.
        const json sent = machine.sent();
        if (expect.contains("sent")) {
            EXPECT_EQ(sent, expect["sent"]) << "what the machine sent its peers";
        }
        // A machine that ended in a top-level <final> has no saved state to read — the save
        // refuses one — so what a scenario can say of it is that it ended, and the data its
        // <donedata> left, compared as a value.
        if (expect.value("ended", false)) {
            EXPECT_FALSE(expect.contains("state") || expect.contains("variables"))
                << "an ended machine has no state or variables to read";
            EXPECT_TRUE(machine.ended()) << "the machine ended in a top-level <final>";
            if (expect.contains("donedata")) {
                EXPECT_EQ(json::parse(machine.donedata()), expect["donedata"])
                    << "the data the final's done event carries";
            }
            continue;
        }
        const json saved = machine.saved();
        if (expect.contains("state")) {
            EXPECT_EQ(saved["current"], expect["state"]) << "the current state";
        }
        // A set: the order a machine lists its active states in is not part of the answer.
        if (expect.contains("configuration")) {
            EXPECT_EQ(saved["configuration"].get<std::set<std::string>>(),
                      expect["configuration"].get<std::set<std::string>>())
                << "the active states";
        }
        if (expect.contains("variables")) {
            for (const auto &[variable, want] : expect["variables"].items()) {
                const json got = saved["variables"].value(variable, json());
                EXPECT_TRUE(holds(got, want)) << "variable '" << variable << "' is " << got << ", not " << want;
            }
        }
    }
}

}  // namespace SCE::Tests::StaticScenario
