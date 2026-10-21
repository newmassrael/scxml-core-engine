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
//   * `static_host_call`: a host action takes the machine's variables as typed
//     arguments, and `static_host_call_arguments` (beside this file): an
//     argument that overflows stops the call and raises `error.execution`.
//   * `static_host_duty` (docs/adr/0038): a machine that states a duty takes
//     a host that says whether it performs the duty, and refuses one that only
//     simulates it unless it is built for one on purpose.

#include "common/SceClock.h"
#include "presence_peer_sm.h"
#include "record_list_parent_sm.h"
#include "record_list_sm.h"
#include "sheet_peer_sm.h"
#include "static_block_ends_list_sm.h"
#include "static_block_ends_sm.h"
#include "static_bytes_sm.h"
#include "static_bytes_wire_sm.h"
#include "static_cancel_expr_sm.h"
#include "static_child_host_hybrid_sm.h"
#include "static_child_host_sm.h"
#include "static_child_mesh_sm.h"
#include "static_counter_sm.h"
#include "static_donedata_content_sm.h"
#include "static_donedata_record_sm.h"
#include "static_donedata_sm.h"
#include "static_enum_sm.h"
#include "static_event_arrival_sm.h"
#include "static_event_wildcard_sm.h"
#include "static_foreach_sm.h"
#include "static_history_sm.h"
#include "static_host_call_arguments_sm.h"
#include "static_host_call_sm.h"
#include "static_host_duty_child_sm.h"
#include "static_host_duty_sm.h"
#include "static_invoke_hybrid_saved_sm.h"
#include "static_invoke_hybrid_sm.h"
#include "static_invoke_params_sm.h"
#include "static_invoke_sm.h"
#include "static_invoke_string_sm.h"
#include "static_list_assign_sm.h"
#include "static_list_index_sm.h"
#include "static_list_sm.h"
#include "static_overflow_sm.h"
#include "static_payload_bytes_sm.h"
#include "static_payload_enum_sm.h"
#include "static_payload_relay_sm.h"
#include "static_payload_sm.h"
#include "static_real32_sm.h"
#include "static_real_sm.h"
#include "static_record_bytes_sm.h"
#include "static_record_enum_sm.h"
#include "static_record_fields_sm.h"
#include "static_record_list_index_sm.h"
#include "static_record_list_sm.h"
#include "static_record_real32_sm.h"
#include "static_record_real_sm.h"
#include "static_record_sm.h"
#include "static_record_string_sm.h"
#include "static_schema_revision_sm.h"
#include "static_send_content_sm.h"
#include "static_send_delay_sm.h"
#include "static_send_event_sm.h"
#include "static_send_http_sm.h"
#include "static_send_idlocation_sm.h"
#include "static_send_namelist_sm.h"
#include "static_send_params_sm.h"
#include "static_send_target_sm.h"
#include "static_send_type_sm.h"
#include "static_state_algorithm_enum_sm.h"
#include "static_state_algorithm_sm.h"
#include "static_string_capacity_sm.h"
#include "static_text_record_sm.h"
#include "static_timers_sm.h"
#include "static_whole_payload_sm.h"
#include "static_wire_enum_sm.h"
#include "static_wire_wide_sm.h"
#include "sync_client_sm.h"
#include "todo_peer_sm.h"

#include <cstdint>
#include <deque>
#include <filesystem>
#include <fstream>
#include <functional>
#include <gtest/gtest.h>
#include <map>
#include <memory>
#include <nlohmann/json.hpp>
#include <set>
#include <sstream>
#include <stdexcept>
#include <string>
#include <type_traits>
#include <utility>
#include <vector>

#ifndef SCE_PROJECT_ROOT
#define SCE_PROJECT_ROOT "."
#endif

namespace SCE {
namespace Tests {

namespace {

using json = nlohmann::json;

const std::string kStaticFixtures = std::string(SCE_PROJECT_ROOT) + "/sce-build/tests/fixtures/static_datamodel";

/// A byte string as its byte-exact Latin-1 text, each byte the character of that
/// code point (docs/adr/0005, decision 2), written in the UTF-8 a JSON string is
/// held in: a byte past 0x7F is the two-byte sequence of its code point.
json latin1Text(const std::vector<std::uint8_t> &bytes) {
    std::string text;
    for (const std::uint8_t byte : bytes) {
        if (byte < 0x80) {
            text.push_back(static_cast<char>(byte));
        } else {
            text.push_back(static_cast<char>(0xC0 | (byte >> 6)));
            text.push_back(static_cast<char>(0x80 | (byte & 0x3F)));
        }
    }
    return text;
}

json readScenario(const std::string &machine) {
    const auto path = std::filesystem::path(kStaticFixtures) / "scenarios" / (machine + ".json");
    std::ifstream in(path);
    EXPECT_TRUE(in.is_open()) << "not readable: " << path;
    return json::parse(in);
}

/// How many rounds a host gives a machine that needs `tick` after an event before
/// it reads the machine back: two for each level of child session, and a round
/// that finds nothing to do changes nothing.
constexpr int kSettleTicks = 5;

/// The rounds `machine` needs before it is read back: `tick` for a machine whose
/// policy needs it (a delayed send, a child session), `step` for one that does not.
template <typename Machine> void settleRounds(Machine &machine) {
    if constexpr (::SCE::Core::NeedsEventScheduler<typename Machine::PolicyType>) {
        for (int round = 0; round < kSettleTicks; ++round) {
            machine.tick();
        }
    } else {
        machine.step();
    }
}

/// The child hosts a machine's host was asked for, in the order asked, and what each
/// child asked of its host. A child owns nothing of the host it was given, the
/// machine's host does, so each host is opened here and writes into the book this
/// reads. The children are kept in a deque, whose elements stay put, because a host
/// holds a reference to its own.
class ChildHostBook {
public:
    class Child {
    public:
        Child(std::string invoke, int start, std::string candidate)
            : invoke_(std::move(invoke)), start_(start), candidate_(std::move(candidate)) {}

        /// The child asked its host for `action`, with `args` in the order the
        /// document gives them.
        void call(const std::string &action, json args = json::array()) {
            calls_.push_back({{"action", action}, {"args", std::move(args)}});
        }

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
    /// `candidate` is the stem of the document when the invoke names one of several,
    /// and empty for a child written inline.
    Child &open(const std::string &invoke, const std::string &candidate = "") {
        int start = 1;
        for (const auto &child : children_) {
            if (child.invoke_ == invoke) {
                ++start;
            }
        }
        return children_.emplace_back(invoke, start, candidate);
    }

    /// The children asked for or called since this was last called, oldest asked
    /// first, each as a scenario's `child_hosts` writes it: the invoke, the start of
    /// that invoke, the candidate's document stem where the invoke names one, and
    /// the calls made in this step alone. A child asked for earlier that called
    /// nothing since has no entry.
    json take() {
        json fresh = json::array();
        for (auto &child : children_) {
            json calls = json::array();
            for (; child.taken_ < child.calls_.size(); ++child.taken_) {
                calls.push_back(child.calls_[child.taken_]);
            }
            if (child.announced_ && calls.empty()) {
                continue;
            }
            child.announced_ = true;
            json entry = {{"invoke", child.invoke_}, {"start", child.start_}};
            if (!child.candidate_.empty()) {
                entry["candidate"] = child.candidate_;
            }
            entry["host_calls"] = std::move(calls);
            fresh.push_back(std::move(entry));
        }
        return fresh;
    }

private:
    std::deque<Child> children_;
};

/// The routers a replay gives the child sessions of the machine it drives
/// (SCE_MESH.md, Child Sessions in Mesh; docs/adr/0043). A child that sends to a
/// Mesh peer is given a router of its own by its parent, which asks the factory
/// its host registered; a replay is the host, so it registers one (factory())
/// whose routers write down what each child sent, told by the path of invoke ids
/// that started it and how many children that path had started, and hold what a
/// step's `invoke` delivers to the child until its parent applies it. A router
/// answers for the child's own children in turn, so a grandchild is told by
/// `<invoke>/<invoke>`. The C++ half of the Rust replay's `child_mesh::Children`.
class ChildRouters {
public:
    /// Children whose sends are written into `sent`, beside the machine's own.
    explicit ChildRouters(std::shared_ptr<json> sent) : book_(std::make_shared<Book>()) {
        book_->sent = std::move(sent);
    }

    /// The factory a replay registers on the machine: every child is given a
    /// router, whichever invoke starts it and from whatever document.
    ::SCE::Static::ChildMeshRouters factory() const {
        return [book = book_](const std::string &invokeId, const std::string &) { return start(book, invokeId); };
    }

    /// Hand `event`, with `data` as its `_event.data`, to the running child of
    /// `path`, as its parent would be handed it: the parent applies it the next
    /// time it drives the child. False when no child of `path` is running, which a
    /// step that names one fails on.
    bool deliver(const std::string &path, const std::string &event, const std::string &data) const {
        const auto running = book_->running.find(path);
        if (running == book_->running.end()) {
            return false;
        }
        running->second->push_back(::SCE::Common::ForwardedEvent{event, data, "", "", "external", "", ""});
        return true;
    }

private:
    /// What a child's router holds for it until its parent applies it.
    using Inbox = std::shared_ptr<std::vector<::SCE::Common::ForwardedEvent>>;

    /// The list every send is written into, the starts of each path, and the
    /// inbox of the session each path is running.
    struct Book {
        std::shared_ptr<json> sent;
        std::map<std::string, int> starts;
        std::map<std::string, Inbox> running;
    };

    /// The router of one start of one child.
    class Recording : public ::SCE::Static::IChildMeshRouter {
    public:
        Recording(std::shared_ptr<Book> book, std::string path, int number, Inbox inbox)
            : book_(std::move(book)), path_(std::move(path)), number_(number), inbox_(std::move(inbox)) {}

        /// Written down as the machine's own sends are, with the child's path and
        /// start; the peer is not there, so nothing answers.
        bool send(const std::string &target, const std::string &eventName, const std::string &data, const std::string &,
                  const std::string &) override {
            json entry = {{"target", target}, {"event", eventName}};
            if (!data.empty()) {
                entry["data"] = json::parse(data);
            }
            entry["invoke"] = path_;
            entry["start"] = number_;
            book_->sent->push_back(std::move(entry));
            return true;
        }

        std::vector<::SCE::Common::ForwardedEvent> takeCalls() override {
            return std::exchange(*inbox_, {});
        }

        /// The path is no longer running this session: a step that names it fails
        /// until the path starts a child again.
        void detach() override {
            const auto running = book_->running.find(path_);
            if (running != book_->running.end() && running->second == inbox_) {
                book_->running.erase(running);
            }
        }

        std::shared_ptr<::SCE::Static::IChildMeshRouter> childRouter(const std::string &invokeId,
                                                                     const std::string &) override {
            return start(book_, path_ + "/" + invokeId);
        }

    private:
        std::shared_ptr<Book> book_;
        std::string path_;
        int number_;
        Inbox inbox_;
    };

    /// A router for a new start of `path`, which is now the session the path is
    /// running.
    static std::shared_ptr<::SCE::Static::IChildMeshRouter> start(const std::shared_ptr<Book> &book,
                                                                  const std::string &path) {
        auto inbox = std::make_shared<std::vector<::SCE::Common::ForwardedEvent>>();
        const int number = ++book->starts[path];
        book->running[path] = inbox;
        return std::make_shared<Recording>(book, path, number, std::move(inbox));
    }

    std::shared_ptr<Book> book_;
};

/// One generated machine as a scenario sees it: events by their document name,
/// and the published variables by theirs.
template <typename Machine> class Driver {
public:
    using Reader = std::function<json(const Machine &)>;

    /// What the machine asked of its host since this was last called, oldest first,
    /// each as a scenario's `host_calls` writes it: `{"action": …, "args": […]}`,
    /// the name the document gives the action and its arguments in the order the
    /// document gives them.
    using HostCalls = std::function<json()>;

    /// The child hosts the machine's host was asked for since this was last called,
    /// and what each child asked of its host, each as a scenario's `child_hosts`
    /// writes it.
    using ChildHosts = std::function<json()>;

    explicit Driver(std::map<std::string, Reader> variables) : variables_(std::move(variables)) {}

    /// A machine with `<sce:action>`s takes the host that performs them as it is
    /// built, because its first entry can already call it; `hostCalls` reads what
    /// that host recorded.
    template <typename Host>
    Driver(std::map<std::string, Reader> variables, Host &host, HostCalls hostCalls)
        : machine_(host), variables_(std::move(variables)), hostCalls_(std::move(hostCalls)) {}

    /// A machine that states a duty, driven by a host that only records
    /// (docs/adr/0038): it is built through the opt-in a simulated host needs,
    /// named at the call, and `hostCalls` reads what the host recorded.
    template <typename Host>
    Driver(std::map<std::string, Reader> variables, Host &host, HostCalls hostCalls, ::SCE::SimulatedHost simulated)
        : machine_(host, simulated), variables_(std::move(variables)), hostCalls_(std::move(hostCalls)) {}

    /// A machine that declares no act of its own and whose children are given
    /// hosts: `host` answers each child's, and writes it into `children`, which is
    /// what the scenario's `child_hosts` are read from.
    template <typename Host>
    Driver(std::map<std::string, Reader> variables, Host &host, ChildHostBook &children)
        : machine_(host), variables_(std::move(variables)), childHosts_([&children] { return children.take(); }) {}

    /// The machine runs on a manual clock, so a delayed send waits exactly the
    /// time a step names (`advance`) and not the time the test happened to take.
    ///
    /// What the machine sends a Mesh peer (`<send target="#name">`) is handed to
    /// the callback its host registers, and a machine with none has the send
    /// refused as error.execution (a test below holds it to that). A replay is every machine's host, so it
    /// registers one that only writes the send down and accepts it: the peer is
    /// not there, and what a scenario holds a step to is that the machine asked
    /// for the send. It is registered before the machine starts, because the
    /// first entry can already send.
    ///
    /// A scenario that says `"router": false` is of a machine whose host attached no
    /// link (docs/adr/0046, section 4): `routed` is false, the machine is given no
    /// callback and no factory for its children, and every send to a peer is
    /// refused where it is made.
    void start(bool routed = true) {
        machine_.setClock(std::make_shared<SCE::ManualClock>(0));
        if (routed) {
            machine_.setMeshSendCallback([sent = sent_](const std::string &target, const std::string &event,
                                                        const std::string &data, const std::string &,
                                                        const std::string &) {
                json send = {{"target", target}, {"event", event}};
                if (!data.empty()) {
                    send["data"] = json::parse(data);
                }
                sent->push_back(std::move(send));
                return true;
            });
            // A child session the machine starts is given a router of its own by
            // the factory its host registers (SCE_MESH.md, Child Sessions in Mesh),
            // never the machine's: what a child sends is written into the same list,
            // told by the invoke that started it and the start, and a step's
            // `invoke` hands its event to the child through it.
            machine_.registerChildMeshRouters(children_.factory());
        }
        machine_.initialize();
        // A machine that starts a child session in its first state has that child
        // running, and maybe ended, before any event arrives; it is read as the
        // events leave it, after the rounds that take.
        if constexpr (::SCE::Core::NeedsEventScheduler<typename Machine::PolicyType>) {
            settle();
        }
    }

    /// The machine's time moves on by `ms`, and what that made due runs.
    void advance(uint64_t ms) {
        machine_.advanceTimeMs(ms);
    }

    /// An event goes in, and the macrostep it starts runs to the end. `data` is
    /// the event's payload as JSON text, from which the machine lifts the typed
    /// fields its schema names.
    void send(const std::string &name, const std::string &data = "") {
        machine_.raiseExternal(name, data);
        settle();
    }

    /// A peer's event for the running child of `path` (invoke ids joined by `/`),
    /// handed to the router the child was given; the machine applies it when it
    /// next drives the child, in the rounds that follow. False, and nothing run,
    /// when no child of `path` is running.
    bool sendToChild(const std::string &path, const std::string &name, const std::string &data = "") {
        if (!children_.deliver(path, name, data)) {
            return false;
        }
        settle();
        return true;
    }

    /// Whether an event arriving under `name` reaches the machine at all, as the
    /// engine decides it (§scxml-3.12.1): false is a name it drops.
    bool resolves(const std::string &name) const {
        return machine_.resolveEventByName(name).has_value();
    }

    std::string state() const {
        return Machine::PolicyType::getStateName(machine_.getCurrentState());
    }

    /// Every active state, a compound or a parallel one with the atomic ones below
    /// it, by the id the document gives it.
    std::set<std::string> configuration() const {
        std::set<std::string> ids;
        for (const auto &active : machine_.getActiveStates()) {
            ids.insert(Machine::PolicyType::getStateName(active));
        }
        return ids;
    }

    bool ended() const {
        return machine_.isInFinalState();
    }

    /// The data the top-level final's `<donedata>` left for the invoking parent,
    /// as the JSON text the done event carries.
    std::string donedata() const {
        return machine_.donedataAtFinal();
    }

    json variable(const std::string &name) const {
        const auto found = variables_.find(name);
        EXPECT_NE(found, variables_.end()) << "the driver publishes no variable '" << name << "'";
        return found == variables_.end() ? json() : found->second(machine_);
    }

    /// Whether the machine's host is one that records what it is asked.
    bool recordsHostCalls() const {
        return static_cast<bool>(hostCalls_);
    }

    /// What the machine asked of its host since this was last called. Null for a
    /// machine whose host is not one that records.
    json hostCalls() {
        return hostCalls_ ? hostCalls_() : json();
    }

    /// Whether the machine's children are given hosts that record what they are asked.
    bool recordsChildHosts() const {
        return static_cast<bool>(childHosts_);
    }

    /// The child hosts asked for since this was last called, and what their
    /// children asked of them. Null for a machine whose children's hosts are not
    /// ones that record.
    json childHosts() {
        return childHosts_ ? childHosts_() : json();
    }

    /// What the machine sent its Mesh peers since this was last called, oldest
    /// first, each as a scenario's `sent` writes it: `{"target": …, "event": …,
    /// "data": …}`, the target as the document writes it, and the data the JSON
    /// value of the text the send carried, left out when it carried none.
    json sent() {
        json taken = std::move(*sent_);
        *sent_ = json::array();
        return taken;
    }

private:
    /// The rounds the machine needs before it is read back. The generated policy
    /// says which call it needs: `step` drains the queues and nothing else, and a
    /// machine with a delayed send or a child session is driven by `tick`, which
    /// also runs those. The engine reports no point at which it has settled, so a
    /// host runs rounds: a child takes an event the parent forwarded in one round
    /// and its end reaches the parent in the next.
    void settle() {
        settleRounds(machine_);
    }

    Machine machine_;
    std::map<std::string, Reader> variables_;
    HostCalls hostCalls_;
    ChildHosts childHosts_;
    /// Shared with the callback the machine holds, so that it stays the list the
    /// machine writes to wherever the driver is moved.
    std::shared_ptr<json> sent_ = std::make_shared<json>(json::array());
    /// The routers of the machine's children, which write into the same list.
    ChildRouters children_{sent_};
};

/// Replay `machine`'s scenario file against `driver`. Every step names an event
/// (or none, for the machine as started) and what it must hold afterwards. A name
/// no event of the machine matches is a misspelt step unless the step says it
/// expects the drop (`"dropped": true`), which is then what is held.
template <typename Machine> void replay(const std::string &machine, Driver<Machine> &driver) {
    const json scenario = readScenario(machine);
    ASSERT_TRUE(scenario.contains("steps")) << machine;
    driver.start(scenario.value("router", true));
    int index = 0;
    for (const auto &step : scenario["steps"]) {
        SCOPED_TRACE(machine + " step " + std::to_string(index++) + ": " + step.value("note", std::string{}));
        if (step.contains("advance_ms")) {
            driver.advance(step["advance_ms"].get<uint64_t>());
        }
        if (step.contains("event")) {
            const auto event = step["event"].get<std::string>();
            const std::string data = step.contains("data") ? step["data"].dump() : "";
            if (step.contains("invoke")) {
                // A peer's event for the running child of that invoke, handed to
                // the router the child was given. The child's events are not the
                // machine's, so the machine is not asked whether it matches.
                const auto path = step["invoke"].get<std::string>();
                EXPECT_TRUE(driver.sendToChild(path, event, data))
                    << "no child of '" << path << "' is running to be sent '" << event << "'";
            } else {
                const bool dropped = step.value("dropped", false);
                EXPECT_EQ(driver.resolves(event), !dropped)
                    << "the machine's events " << (dropped ? "match" : "do not match") << " '" << event << "'";
                driver.send(event, data);
            }
        }
        const auto &expect = step["expect"];
        // What the machine asked of its host in this step. Read on every step so
        // that a step which states nothing does not hand its calls to the next, and
        // a step that states some needs a driver that records them.
        const json asked = driver.hostCalls();
        if (expect.contains("host_calls")) {
            EXPECT_TRUE(driver.recordsHostCalls())
                << "this driver records no host calls, so it cannot hold a step to them";
            EXPECT_EQ(asked, expect["host_calls"]) << "what the machine asked of its host";
        }
        // The child hosts the machine's host was asked for in this step, and what
        // each child asked of its host, taken and held the same way.
        const json children = driver.childHosts();
        if (expect.contains("child_hosts")) {
            EXPECT_TRUE(driver.recordsChildHosts())
                << "this driver records no child hosts, so it cannot hold a step to them";
            EXPECT_EQ(children, expect["child_hosts"])
                << "the child hosts the machine's host was asked for, and what their children asked of them";
        }
        // What the machine sent its Mesh peers in this step, taken on every step
        // for the same reason. Every driver records them, so a step that states
        // some is always held to them; compared as values, in the order sent.
        const json sent = driver.sent();
        if (expect.contains("sent")) {
            EXPECT_EQ(sent, expect["sent"]) << "what the machine sent its peers";
        }
        if (expect.value("ended", false)) {
            EXPECT_TRUE(driver.ended());
            // Compared as a value, so the order the pairs were written in is
            // not part of the answer.
            if (expect.contains("donedata")) {
                EXPECT_EQ(json::parse(driver.donedata()), expect["donedata"])
                    << "the data the final's done event carries";
            }
            continue;
        }
        if (expect.contains("state")) {
            EXPECT_EQ(driver.state(), expect["state"].get<std::string>());
        }
        // A set: the order a machine lists its active states in is not part of
        // the answer.
        if (expect.contains("configuration")) {
            const auto want = expect["configuration"].get<std::set<std::string>>();
            EXPECT_EQ(driver.configuration(), want) << "the active states";
        }
        if (expect.contains("variables")) {
            for (const auto &[name, value] : expect["variables"].items()) {
                EXPECT_EQ(driver.variable(name), value) << "variable '" << name << "'";
            }
        }
    }
}

/// A `record:Day` value as a scenario states it: its fields by the schema's ids.
template <typename Day> json dayJson(const Day &day) {
    return json{{"year", day.year}, {"month", day.month}, {"dayOfMonth", day.dayOfMonth}};
}

/// A `record:Entry` value as a scenario states it: its fields by the schema's ids.
template <typename Entry> json entryJson(const Entry &entry) {
    return json{{"id", entry.id}, {"amount", entry.amount}};
}

/// A list of such values, in order.
template <typename Entries> json entriesJson(const Entries &entries) {
    json listed = json::array();
    for (const auto &entry : entries) {
        listed.push_back(entryJson(entry));
    }
    return listed;
}

/// A `record:Total` value as a scenario states it: its fields by the schema's ids.
template <typename Total> json totalJson(const Total &total) {
    return json{{"count", total.count}, {"sum", total.sum}};
}

/// A `record:Reading` value as a scenario states it: its fields by the schema's ids.
template <typename Reading> json readingJson(const Reading &reading) {
    return json{{"sensor", reading.sensor}, {"value", reading.value}};
}

/// A list of such values, in order.
template <typename Days> json daysJson(const Days &days) {
    json listed = json::array();
    for (const auto &day : days) {
        listed.push_back(dayJson(day));
    }
    return listed;
}

/// A `record:Framed` value as a scenario states it: its fields by the schema's ids,
/// the byte string as its byte-exact Latin-1 text.
template <typename Framed> json framedJson(const Framed &framed) {
    return json{{"sensor", framed.sensor}, {"frame", latin1Text(framed.frame)}};
}

/// A list of such values, in order.
template <typename Frameds> json framedsJson(const Frameds &frameds) {
    json listed = json::array();
    for (const auto &framed : frameds) {
        listed.push_back(framedJson(framed));
    }
    return listed;
}

/// A `record:Labelled` value as a scenario states it: its fields by the schema's ids.
template <typename Labelled> json labelledJson(const Labelled &labelled) {
    return json{{"sensor", labelled.sensor}, {"label", labelled.label}};
}

/// A list of such values, in order.
template <typename Labelleds> json labelledsJson(const Labelleds &labelleds) {
    json listed = json::array();
    for (const auto &labelled : labelleds) {
        listed.push_back(labelledJson(labelled));
    }
    return listed;
}

/// A 64-bit integer as a scenario of the shared copies states it: the text of
/// its digits, which is how a saved state writes one (a JSON number past 2^53 is
/// not exact everywhere).
template <typename Wide> json wideText(Wide value) {
    static_assert(sizeof(Wide) == 8 && std::is_integral_v<Wide>, "a 64-bit integer");
    return std::to_string(value);
}

/// A `record:Slot` value as a scenario states it: its fields by the schema's ids,
/// the version as the text of its digits.
template <typename Slot> json slotJson(const Slot &slot) {
    return json{{"slot", slot.slot}, {"version", wideText(slot.version)}, {"text", slot.text}};
}

/// A list of such values, in order.
template <typename Slots> json slotsJson(const Slots &slots) {
    json listed = json::array();
    for (const auto &slot : slots) {
        listed.push_back(slotJson(slot));
    }
    return listed;
}

/// Each element of a list as `one` writes it, in order.
template <typename List, typename One> json listedJson(const List &list, One one) {
    json listed = json::array();
    for (const auto &element : list) {
        listed.push_back(one(element));
    }
    return listed;
}

/// A machine that publishes a variable named `clock`. The engine already has a member of
/// that name (`StaticExecutionEngine::clock()`), so the generator gives the variable no
/// reader of its own; the value is read from the policy that holds it.
template <typename Machine> class ClockPeeking : public Machine {
public:
    using Machine::Machine;

    const auto &publishedClock() const {
        return this->policy_.v_clock;
    }
};

/// The policy that holds a machine's variables. A published variable the machine
/// already has a member by, such as `clock`, which is the engine's own, is given no
/// reader (`reader_names::reader_is_taken`), so a host that must see it reads the field
/// off the policy that holds it.
template <typename Machine> auto &policyOf(const Machine &machine) {
    return const_cast<Machine &>(machine).getPolicy();
}

/// A `record:Hlc` value (sce:std/merge/hlc_timestamp) as a scenario states it.
template <typename Hlc> json hlcJson(const Hlc &stamp) {
    return json{{"wallTime", wideText(stamp.wallTime)}, {"counter", stamp.counter}, {"nodeId", wideText(stamp.nodeId)}};
}

/// A list of `record:LwwCell` values (sce:std/merge/lww_cell), in order.
template <typename Cells> json lwwCellsJson(const Cells &cells) {
    return listedJson(cells, [](const auto &cell) {
        return json{{"row", wideText(cell.row)},
                    {"col", wideText(cell.col)},
                    {"value", wideText(cell.value)},
                    {"wallTime", wideText(cell.wallTime)},
                    {"counter", cell.counter},
                    {"nodeId", wideText(cell.nodeId)},
                    {"op", cell.op},
                    {"aRef", cell.aRef},
                    {"aRow", wideText(cell.aRow)},
                    {"aCol", wideText(cell.aCol)},
                    {"aVal", wideText(cell.aVal)},
                    {"bRef", cell.bRef},
                    {"bRow", wideText(cell.bRow)},
                    {"bCol", wideText(cell.bCol)},
                    {"bVal", wideText(cell.bVal)}};
    });
}

/// A list of `record:Entry` values (sce:std/merge/orset_entry), in order.
template <typename Entries> json orsetEntriesJson(const Entries &entries) {
    return listedJson(entries, [](const auto &entry) {
        return json{{"element", wideText(entry.element)},
                    {"wallTime", wideText(entry.wallTime)},
                    {"counter", entry.counter},
                    {"nodeId", wideText(entry.nodeId)}};
    });
}

/// A list of `record:Shown` values (sce:std/merge/sheet_shown), in order.
template <typename Shown> json sheetShownJson(const Shown &shown) {
    return listedJson(shown, [](const auto &record) {
        return json{{"row", wideText(record.row)},
                    {"col", wideText(record.col)},
                    {"value", wideText(record.value)},
                    {"fault", record.fault}};
    });
}

/// A list of `record:Value` values (sce:std/merge/mv_value), in order.
template <typename Values> json todoValuesJson(const Values &values) {
    return listedJson(values, [](const auto &held) {
        return json{{"id", wideText(held.id)},
                    {"field", wideText(held.field)},
                    {"value", wideText(held.value)},
                    {"text", held.text},
                    {"wallTime", wideText(held.wallTime)},
                    {"counter", held.counter},
                    {"nodeId", wideText(held.nodeId)}};
    });
}

/// A list of `record:Node` values (sce:std/merge/merkle_node), in order: one node of a digest.
template <typename Nodes> json merkleNodesJson(const Nodes &nodes) {
    return listedJson(nodes, [](const auto &node) {
        return json{{"depth", node.depth}, {"prefix", wideText(node.prefix)}, {"hash", node.hash}};
    });
}

/// A list of `record:RgaNode` values (sce:std/merge/rga_node), in order.
template <typename Nodes> json rgaNodesJson(const Nodes &nodes) {
    return listedJson(
        nodes, [](const auto &node) { return json{{"key", wideText(node.key)}, {"after", wideText(node.after)}}; });
}

/// A list of `record:RgaGone` values (sce:std/merge/rga_gone), in order.
template <typename Gones> json rgaGonesJson(const Gones &gones) {
    return listedJson(gones,
                      [](const auto &gone) { return json{{"key", wideText(gone.key)}, {"by", wideText(gone.by)}}; });
}

/// A list of `record:Seen` (presence_seen.scxml), in order: a client an entry of the
/// list of who is online is for, the latest heartbeat stamp the copy holds for it, and
/// the renewal of the entry that stamp belongs to.
template <typename Entries> json presenceEntriesJson(const Entries &entries) {
    return listedJson(entries, [](const auto &entry) {
        return json{{"client", entry.client}, {"last", wideText(entry.last)}, {"gen", entry.gen}};
    });
}

/// A list of `record:Occurrence` (event_occurrence.scxml), in order: one occurrence of a
/// scheduled to-do in an agenda, its start and end as instants.
template <typename Occurrences> json occurrencesJson(const Occurrences &occurrences) {
    return listedJson(occurrences, [](const auto &occurrence) {
        return json{{"id", wideText(occurrence.id)},
                    {"start", wideText(occurrence.start)},
                    {"end", wideText(occurrence.end)},
                    {"nth", occurrence.nth},
                    {"conflict", occurrence.conflict}};
    });
}

/// A `record:Register` (sce:std/shared/record_register): what a shared record list told
/// the statechart that invoked it of one register, its count and its first live value
/// with the dot of its write.
template <typename Register> json registerJson(const Register &told) {
    return json{{"id", told.id},           {"field", told.field},
                {"count", told.count},     {"value", wideText(told.value)},
                {"text", told.text},       {"wallTime", wideText(told.wallTime)},
                {"counter", told.counter}, {"nodeId", wideText(told.nodeId)}};
}

/// A list of `record:RegisterValue` (sce:std/shared/record_value), in order: the
/// further live values of a register, each with its place in the order of the dots.
template <typename Values> json registerValuesJson(const Values &values) {
    return listedJson(values, [](const auto &told) {
        return json{{"id", told.id},           {"field", told.field},
                    {"ordinal", told.ordinal}, {"value", wideText(told.value)},
                    {"text", told.text},       {"wallTime", wideText(told.wallTime)},
                    {"counter", told.counter}, {"nodeId", wideText(told.nodeId)}};
    });
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

/// An event arrives by name from outside the document, so the names it can arrive
/// under are open (§scxml-3.12.1): a name the document never writes reaches the
/// transition whose descriptor is a token prefix of it, and one no descriptor
/// matches is dropped.
TEST(AStaticDatamodelRunsGeneratedCppTest, AnEventArrivesUnderANameTheDocumentDoesNotWrite) {
    using Machine = G::static_event_arrival::static_event_arrival;
    Driver<Machine> driver({
        {"requests", [](const Machine &m) { return json(m.requests()); }},
        {"specials", [](const Machine &m) { return json(m.specials()); }},
    });
    replay("static_event_arrival", driver);
}

/// ...and where the document listens with `event="*"`, a name no descriptor it
/// writes extends is delivered as the wildcard event instead of being dropped.
TEST(AStaticDatamodelRunsGeneratedCppTest, AnEventArrivesUnderANameOnlyTheWildcardTakes) {
    using Machine = G::static_event_wildcard::static_event_wildcard;
    Driver<Machine> driver({
        {"requests", [](const Machine &m) { return json(m.requests()); }},
        {"strays", [](const Machine &m) { return json(m.strays()); }},
    });
    replay("static_event_wildcard", driver);
}

TEST(AStaticDatamodelRunsGeneratedCppTest, AnOverflowingOperationFailsInsteadOfWrapping) {
    using Machine = G::static_overflow::static_overflow;
    Driver<Machine> driver({
        {"level", [](const Machine &m) { return json(m.level()); }},
        {"refusals", [](const Machine &m) { return json(m.refusals()); }},
    });
    replay("static_overflow", driver);
}

// An event's typed payload is read in a guard and in assignments, and a
// computation that does not fit skips its assignment and says so.
TEST(AStaticDatamodelRunsGeneratedCppTest, AnEventsTypedPayloadIsReadInAGuardAndInContent) {
    using Machine = G::static_payload::static_payload;
    Driver<Machine> driver({
        {"day", [](const Machine &m) { return json(m.day()); }},
        {"late", [](const Machine &m) { return json(m.late()); }},
        {"sinceEpoch", [](const Machine &m) { return json(m.sinceEpoch()); }},
        {"refusals", [](const Machine &m) { return json(m.refusals()); }},
    });
    replay("static_payload", driver);
}

// An enum variable holds a variant of its enum, is compared with `===` and
// `!==` to a variant or to another variable of the same enum, and takes a
// conditional of two variants. The scenario names a value as the enum document
// does, so the host's enum is read back by `sceLogName`.
TEST(AStaticDatamodelRunsGeneratedCppTest, AnEnumVariableHoldsAVariantOfItsEnum) {
    namespace E = G::static_enum;

    // `previous` is the machine's own, which the scenario states anyway: the
    // probe reaches the policy that holds it.
    struct Probe : E::static_enum {
        const E::static_enumPolicy &policy() const {
            return this->policy_;
        }
    };

    Driver<Probe> driver({
        {"layout", [](const Probe &m) { return json(std::string(E::sceLogName(m.layout()))); }},
        {"previous", [](const Probe &m) { return json(std::string(E::sceLogName(m.policy().v_previous))); }},
        {"changes", [](const Probe &m) { return json(m.changes()); }},
    });
    replay("static_enum", driver);
}

// An event's payload carries an enum field, the variant's declared name: a guard
// compares it to a variant and an assignment stores it in a variable of the enum.
// The scenario names a value as the enum document does, so the host's enum is
// read back by `sceLogName`.
TEST(AStaticDatamodelRunsGeneratedCppTest, AnEventsPayloadCarriesAnEnumField) {
    namespace E = G::static_payload_enum;
    using Machine = E::static_payload_enum;
    Driver<Machine> driver({
        {"layout", [](const Machine &m) { return json(std::string(E::sceLogName(m.layout()))); }},
        {"zoom", [](const Machine &m) { return json(m.zoom()); }},
        {"agenda", [](const Machine &m) { return json(m.agenda()); }},
        {"shown", [](const Machine &m) { return json(m.shown()); }},
    });
    replay("static_payload_enum", driver);
}

// The payload of the event a transition is on is carried on as the `<param>`s
// of a `<send>`: an enum field as the name its enum declares and an integer,
// read where the send runs.
TEST(AStaticDatamodelRunsGeneratedCppTest, ThePayloadOfAnEventIsCarriedOnAsParams) {
    namespace E = G::static_payload_relay;
    using Machine = E::static_payload_relay;
    Driver<Machine> driver({
        {"layout", [](const Machine &m) { return json(std::string(E::sceLogName(m.layout()))); }},
        {"zoom", [](const Machine &m) { return json(m.zoom()); }},
        {"relays", [](const Machine &m) { return json(m.relays()); }},
        {"refusals", [](const Machine &m) { return json(m.refusals()); }},
    });
    replay("static_payload_relay", driver);
}

// A list is a bounded `std::vector`: appended to while it has room, and a full
// list is an error rather than a longer list; `len` measures it and a clear
// empties it.
TEST(AStaticDatamodelRunsGeneratedCppTest, AListIsFilledToItsBoundAndEmptied) {
    using Machine = G::static_list::static_list;
    Driver<Machine> driver({
        {"picked", [](const Machine &m) { return json(m.picked()); }},
        {"refusals", [](const Machine &m) { return json(m.refusals()); }},
        {"count", [](const Machine &m) { return json(m.count()); }},
    });
    replay("static_list", driver);
}

// A list variable takes whole what an algorithm returns as a list; a call that
// fails leaves it as it was and raises error.execution.
TEST(AStaticDatamodelRunsGeneratedCppTest, AListTakesWhatAnAlgorithmReturnsWhole) {
    using Machine = G::static_list_assign::static_list_assign;
    Driver<Machine> driver({
        {"shown", [](const Machine &m) { return json(m.shown()); }},
        {"few", [](const Machine &m) { return json(m.few()); }},
        {"first", [](const Machine &m) { return json(m.first()); }},
        {"count", [](const Machine &m) { return json(m.count()); }},
        {"size", [](const Machine &m) { return json(m.size()); }},
        {"refusals", [](const Machine &m) { return json(m.refusals()); }},
    });
    replay("static_list_assign", driver);
}

// A statechart hands the algorithms the records and lists it holds, and the
// event's own payload, and takes back what they return, written only when the
// call succeeded and held to the bound of the variable it is written to.
TEST(AStaticDatamodelRunsGeneratedCppTest, AStatechartHandsItsStateToAnAlgorithmAndTakesItBack) {
    using Machine = G::static_state_algorithm::static_state_algorithm;
    Driver<Machine> driver({
        {"entries", [](const Machine &m) { return entriesJson(m.entries()); }},
        {"total", [](const Machine &m) { return totalJson(m.total()); }},
        {"last", [](const Machine &m) { return entryJson(m.last()); }},
        {"ids", [](const Machine &m) { return json(m.ids()); }},
        {"idsum", [](const Machine &m) { return json(m.idsum()); }},
        {"large", [](const Machine &m) { return json(m.large()); }},
        {"refusals", [](const Machine &m) { return json(m.refusals()); }},
    });
    replay("static_state_algorithm", driver);
}

// A statechart hands the standard text algorithms the record and the list of
// records it holds, whose text is bounded, and takes back what they return: a
// text past the bound is refused where the payload is taken, the state as it was.
TEST(AStaticDatamodelRunsGeneratedCppTest, AStatechartHandsBoundedTextToAnAlgorithmAndTakesItBack) {
    using Machine = G::static_text_record::static_text_record;
    Driver<Machine> driver({
        {"slots", [](const Machine &m) { return slotsJson(m.slots()); }},
        {"held", [](const Machine &m) { return slotJson(m.held()); }},
        {"incoming", [](const Machine &m) { return slotJson(m.incoming()); }},
        {"known", [](const Machine &m) { return json(m.known()); }},
        {"refusals", [](const Machine &m) { return json(m.refusals()); }},
    });
    replay("static_text_record", driver);
}

// A statechart hands the algorithms records whose field is a value of an enum, and
// a value of the enum itself, and takes back a list of those records and a value of
// the enum: each crosses by the name its variant is declared with, which is what a
// host reads it back as.
TEST(AStaticDatamodelRunsGeneratedCppTest, AStatechartHandsAnEnumToAnAlgorithmAndTakesItBack) {
    namespace E = G::static_state_algorithm_enum;
    using Machine = E::static_state_algorithm_enum;
    Driver<Machine> driver({
        {"items",
         [](const Machine &m) {
             json listed = json::array();
             for (const auto &item : m.items()) {
                 listed.push_back(json{{"id", item.id}, {"status", std::string(E::sceLogName(item.status))}});
             }
             return listed;
         }},
        {"last", [](const Machine &m) { return json(std::string(E::sceLogName(m.last()))); }},
        {"openCount", [](const Machine &m) { return json(m.openCount()); }},
        {"startedCount", [](const Machine &m) { return json(m.startedCount()); }},
        {"doneCount", [](const Machine &m) { return json(m.doneCount()); }},
        {"checks", [](const Machine &m) { return json(m.checks()); }},
        {"refusals", [](const Machine &m) { return json(m.refusals()); }},
    });
    replay("static_state_algorithm_enum", driver);
}

// An element is read by its index like any number; an index below zero or not
// below the length reads nothing and raises error.execution.
TEST(AStaticDatamodelRunsGeneratedCppTest, AListElementIsReadByItsIndex) {
    using Machine = G::static_list_index::static_list_index;
    Driver<Machine> driver({
        {"picked", [](const Machine &m) { return json(m.picked()); }},
        {"cursor", [](const Machine &m) { return json(m.cursor()); }},
        {"under", [](const Machine &m) { return json(m.under()); }},
        {"first", [](const Machine &m) { return json(m.first()); }},
        {"last", [](const Machine &m) { return json(m.last()); }},
        {"ordered", [](const Machine &m) { return json(m.ordered()); }},
        {"room", [](const Machine &m) { return json(m.room()); }},
        {"count", [](const Machine &m) { return json(m.count()); }},
        {"refusals", [](const Machine &m) { return json(m.refusals()); }},
    });
    replay("static_list_index", driver);
}

// A `<foreach>` walks the list as it was when the loop began, binds its item
// and index, and an error in its body ends the block that holds it.
TEST(AStaticDatamodelRunsGeneratedCppTest, AForeachWalksAListVariable) {
    using Machine = G::static_foreach::static_foreach;
    Driver<Machine> driver({
        {"picked", [](const Machine &m) { return json(m.picked()); }},
        {"total", [](const Machine &m) { return json(m.total()); }},
        {"weighted", [](const Machine &m) { return json(m.weighted()); }},
        {"small", [](const Machine &m) { return json(m.small()); }},
        {"crossings", [](const Machine &m) { return json(m.crossings()); }},
        {"visited", [](const Machine &m) { return json(m.visited()); }},
        {"finished", [](const Machine &m) { return json(m.finished()); }},
        {"errors", [](const Machine &m) { return json(m.errors()); }},
    });
    replay("static_foreach", driver);
}

// A 64-bit real is a native double field: a product and a sum, a quotient, a
// guard comparing it with a literal, and a `<foreach>` summing a list of reals.
TEST(AStaticDatamodelRunsGeneratedCppTest, ARealIsANativeBinary64Field) {
    using Machine = G::static_real::static_real;
    Driver<Machine> driver({
        {"level", [](const Machine &m) { return json(m.level()); }},
        {"total", [](const Machine &m) { return json(m.total()); }},
        {"drift", [](const Machine &m) { return json(m.drift()); }},
        {"samples", [](const Machine &m) { return json(m.samples()); }},
        {"errors", [](const Machine &m) { return json(m.errors()); }},
    });
    replay("static_real", driver);
}

// A 32-bit real is a native float field: every operation on it is rounded to
// binary32 where it is made. A host reads it as the double it widens to, which is
// exact, and that is the number the scenario states.
TEST(AStaticDatamodelRunsGeneratedCppTest, ARealIsANativeBinary32Field) {
    using Machine = G::static_real32::static_real32;
    Driver<Machine> driver({
        {"level", [](const Machine &m) { return json(static_cast<double>(m.level())); }},
        {"drift", [](const Machine &m) { return json(static_cast<double>(m.drift())); }},
        {"wide", [](const Machine &m) { return json(m.wide()); }},
        {"total", [](const Machine &m) { return json(static_cast<double>(m.total())); }},
        // A list of floats: each element is stored by the JSON value as the
        // double it widens to.
        {"samples", [](const Machine &m) { return json(m.samples()); }},
    });
    replay("static_real32", driver);
}

TEST(AStaticDatamodelRunsGeneratedCppTest, AnAppendThatFailsEndsItsBlock) {
    using Machine = G::static_block_ends_list::static_block_ends_list;
    Driver<Machine> driver({
        {"picked", [](const Machine &m) { return json(m.picked()); }},
        {"afterAppend", [](const Machine &m) { return json(m.afterAppend()); }},
        {"errors", [](const Machine &m) { return json(m.errors()); }},
    });
    replay("static_block_ends_list", driver);
}

// A record variable is built whole from its `<sce:set>`s, read field by field,
// and updated a field at a time — from the machine's own value and from a typed
// event payload. A field assignment that does not fit is skipped, and the one
// after it in the same block is not processed.
TEST(AStaticDatamodelRunsGeneratedCppTest, ARecordIsBuiltWholeAndUpdatedAFieldAtATime) {
    using Machine = G::static_record_fields::static_record_fields;
    Driver<Machine> driver({
        {"shown", [](const Machine &m) { return dayJson(m.shown()); }},
        {"refusals", [](const Machine &m) { return json(m.refusals()); }},
    });
    replay("static_record_fields", driver);
}

// A record with a 64-bit real field is built whole, written a field at a time,
// and replaced from a typed payload without losing a bit of the real it carried.
TEST(AStaticDatamodelRunsGeneratedCppTest, ARecordHoldsARealFieldToTheBit) {
    using Machine = G::static_record_real::static_record_real;
    Driver<Machine> driver({
        {"last", [](const Machine &m) { return readingJson(m.last()); }},
        {"sum", [](const Machine &m) { return json(m.sum()); }},
    });
    replay("static_record_real", driver);
}

// A record with a 32-bit real field: the double a payload carries lands as the
// single nearest it. A host reads a float as the double it widens to, which the
// JSON value does by itself.
TEST(AStaticDatamodelRunsGeneratedCppTest, ARecordHoldsASingleFieldAsTheSingleNearestThePayload) {
    using Machine = G::static_record_real32::static_record_real32;
    Driver<Machine> driver({
        {"last", [](const Machine &m) { return readingJson(m.last()); }},
        {"sum", [](const Machine &m) { return json(static_cast<double>(m.sum())); }},
        {"errors", [](const Machine &m) { return json(m.errors()); }},
    });
    replay("static_record_real32", driver);
}

// A record's bytes field is held to the bytes its schema declares: an assignment
// past the bound — from a literal or a bytes variable — writes nothing, raises
// `error.execution` and ends its block, and a list of such records holds copies with
// their bytes. A scenario states a byte string as its byte-exact Latin-1 text.
TEST(AStaticDatamodelRunsGeneratedCppTest, ARecordHoldsABytesFieldWithinTheBoundItsSchemaDeclares) {
    using Machine = G::static_record_bytes::static_record_bytes;
    Driver<Machine> driver({
        {"last", [](const Machine &m) { return framedJson(m.last()); }},
        {"spare", [](const Machine &m) { return latin1Text(m.spare()); }},
        {"frames", [](const Machine &m) { return framedsJson(m.frames()); }},
        {"size", [](const Machine &m) { return json(m.size()); }},
        {"matches", [](const Machine &m) { return json(m.matches()); }},
        {"misses", [](const Machine &m) { return json(m.misses()); }},
        {"leads", [](const Machine &m) { return json(m.leads()); }},
        {"errors", [](const Machine &m) { return json(m.errors()); }},
    });
    replay("static_record_bytes", driver);
}

// A host is handed a record by value, and the records of a list as a constant
// reference to the machine's own vector: it can write into the first only into its
// own copy, and into the second not at all. Checked where it is compiled, since a
// reader that returned a mutable reference would be another type.
static_assert(std::is_same_v<decltype(std::declval<const G::static_record_bytes::static_record_bytes &>().last()),
                             G::static_record_bytes::StaticRecordBytesFramedRecord>,
              "a published record is handed out by value");
static_assert(std::is_same_v<decltype(std::declval<const G::static_record_bytes::static_record_bytes &>().frames()),
                             const std::vector<G::static_record_bytes::StaticRecordBytesFramedRecord> &>,
              "a published list of records is lent as a constant reference");

// The bytes an event's payload carries are read into a bytes variable, into a record's
// field and into a whole record, each held to its own bound: a value past it writes
// nothing, raises `error.execution` and ends its block. The wire spells a byte string
// as its byte-exact Latin-1 text, which a scenario states it as.
TEST(AStaticDatamodelRunsGeneratedCppTest, ThePayloadOfAnEventCarriesBytesHeldWithinTheirBounds) {
    using Machine = G::static_payload_bytes::static_payload_bytes;
    Driver<Machine> driver({
        {"last", [](const Machine &m) { return framedJson(m.last()); }},
        {"held", [](const Machine &m) { return latin1Text(m.held()); }},
        {"frames", [](const Machine &m) { return framedsJson(m.frames()); }},
        {"size", [](const Machine &m) { return json(m.size()); }},
        {"matches", [](const Machine &m) { return json(m.matches()); }},
        {"misses", [](const Machine &m) { return json(m.misses()); }},
        {"errors", [](const Machine &m) { return json(m.errors()); }},
    });
    replay("static_payload_bytes", driver);
}

// A record's string field is held to the UTF-8 bytes its schema declares: an
// assignment past the bound — from a literal, a string variable or a payload —
// writes nothing, raises error.execution and ends its block, and a list of such
// records holds copies with their text.
TEST(AStaticDatamodelRunsGeneratedCppTest, ARecordHoldsAStringFieldWithinTheBoundItsSchemaDeclares) {
    using Machine = G::static_record_string::static_record_string;
    Driver<Machine> driver({
        {"last", [](const Machine &m) { return labelledJson(m.last()); }},
        {"note", [](const Machine &m) { return json(m.note()); }},
        {"labels", [](const Machine &m) { return labelledsJson(m.labels()); }},
        {"errors", [](const Machine &m) { return json(m.errors()); }},
    });
    replay("static_record_string", driver);
}

// A byte string crosses a `<param>` and a `<donedata>` as its byte-exact Latin-1 text:
// sent to itself and read back through the typed payload, and carried by the done
// event's data.
TEST(AStaticDatamodelRunsGeneratedCppTest, AByteStringCrossesAParamAsItsLatin1Text) {
    using Machine = G::static_bytes_wire::static_bytes_wire;
    Driver<Machine> driver({
        {"held", [](const Machine &m) { return latin1Text(m.held()); }},
        {"echo", [](const Machine &m) { return latin1Text(m.echo()); }},
        {"relays", [](const Machine &m) { return json(m.relays()); }},
        {"errors", [](const Machine &m) { return json(m.errors()); }},
    });
    replay("static_bytes_wire", driver);
}

// A top-level final's `<donedata>` params are computed from the machine's own
// fields when it is entered, and the pair whose value does not fit is left out.
TEST(AStaticDatamodelRunsGeneratedCppTest, ATopLevelFinalHandsTheDoneEventItsParams) {
    using Machine = G::static_donedata::static_donedata;
    Driver<Machine> driver({
        {"count", [](const Machine &m) { return json(m.count()); }},
    });
    replay("static_donedata", driver);
}

// A top-level final whose `<donedata>` names a record in its `<content expr>` hands
// its done event the pairs of the record's fields, read when the state is entered.
TEST(AStaticDatamodelRunsGeneratedCppTest, ATopLevelFinalHandsTheDoneEventTheRecordItsContentNames) {
    namespace E = G::static_donedata_record;
    using Machine = E::static_donedata_record;
    Driver<Machine> driver({
        {"shown",
         [](const Machine &m) {
             return json{{"layout", std::string(E::sceLogName(m.shown().layout))}, {"zoom", m.shown().zoom}};
         }},
    });
    replay("static_donedata_record", driver);
}

// A top-level final whose `<donedata>` is inline `<content>` hands its done event
// the text as the string it spells, with no script engine to read it as a number.
TEST(AStaticDatamodelRunsGeneratedCppTest, ATopLevelFinalHandsTheDoneEventTheTextItsContentSpells) {
    using Machine = G::static_donedata_content::static_donedata_content;
    Driver<Machine> driver({
        {"count", [](const Machine &m) { return json(m.count()); }},
    });
    replay("static_donedata_content", driver);
}

// A top-level final whose `<donedata>` carries a `<content expr>` that names one
// value hands its done event that value as its whole data: a number as its digits,
// a string quoted, and one that cannot be computed as the empty string.
TEST(AStaticDatamodelRunsGeneratedCppTest, ATopLevelFinalHandsTheDoneEventTheValueItsContentNames) {
    using Machine = G::static_donedata_content::static_donedata_content;
    {
        Driver<Machine> driver({
            {"count", [](const Machine &m) { return json(m.count()); }},
        });
        replay("static_donedata_content_value", driver);
    }
    {
        Driver<Machine> driver({
            {"count", [](const Machine &m) { return json(m.count()); }},
        });
        replay("static_donedata_content_text", driver);
    }
    {
        Driver<Machine> driver({
            {"count", [](const Machine &m) { return json(m.count()); }},
        });
        replay("static_donedata_content_lost", driver);
    }
}

// The `eventexpr` of a `<send>` is a string computed from the machine's fields when
// the send runs, and names the event the send delivers.
TEST(AStaticDatamodelRunsGeneratedCppTest, ASendsEventIsNamedWhenItRuns) {
    using Machine = G::static_send_event::static_send_event;
    Driver<Machine> driver({
        {"pings", [](const Machine &m) { return json(m.pings()); }},
        {"pongs", [](const Machine &m) { return json(m.pongs()); }},
        {"refusals", [](const Machine &m) { return json(m.refusals()); }},
    });
    replay("static_send_event", driver);
}

// The `targetexpr` of a `<send>` is a string computed from the machine's fields when
// the send runs, held to the routes the document declares as `sce:targets`
// (docs/adr/0005, decision 3): the send goes by the entry that matches, and a value
// in none of them is error.communication with nothing sent.
TEST(AStaticDatamodelRunsGeneratedCppTest, ASendsTargetIsChosenAmongTheDeclaredRoutes) {
    using Machine = G::static_send_target::static_send_target;
    Driver<Machine> driver({
        {"landed", [](const Machine &m) { return json(m.landed()); }},
        {"refused", [](const Machine &m) { return json(m.refused()); }},
    });
    replay("static_send_target", driver);
}

// The `typeexpr` of a `<send>` is a string computed from the machine's fields when
// the send runs, held to the processors the document declares as `sce:types`
// (docs/adr/0005, decision 3): the send is delivered by the processor the matching
// entry names, and a value in none of them is error.execution with nothing sent.
TEST(AStaticDatamodelRunsGeneratedCppTest, ASendsTypeIsChosenAmongTheDeclaredProcessors) {
    using Machine = G::static_send_type::static_send_type;
    Driver<Machine> driver({
        {"landed", [](const Machine &m) { return json(m.landed()); }},
        {"refused", [](const Machine &m) { return json(m.refused()); }},
    });
    replay("static_send_type", driver);
}

// The `<param>`s of a BasicHTTP `<send>` are read from the machine's own fields when
// the send runs, and cross as the text a form carries (docs/adr/0005, decision 4,
// §scxml-C-2). The request is observed where the engine hands it to its transport,
// so no listener is involved. A scenario states what a machine's fields hold and not
// what it sent over a wire, so `static_send_http` has no scenario: each engine holds
// it with a test of its own that records the request.
namespace {

using HttpMachine = G::static_send_http::static_send_http;
using HttpParams = std::map<std::string, std::vector<std::string>>;

/// A machine whose transport keeps what it was handed.
struct HttpHarness {
    HttpMachine sm;
    std::vector<SCE::Static::HttpSendRequest> posted;

    HttpHarness() {
        sm.setHttpSendCallback([this](const SCE::Static::HttpSendRequest &request) { posted.push_back(request); });
        sm.initialize();
        sm.step();
    }

    void raise(const std::string &event) {
        sm.raiseExternal(event);
        sm.step();
    }
};

}  // namespace

TEST(AStaticDatamodelRunsGeneratedCppTest, ASendOverHttpCarriesTheTextTheFieldsHoldWhenItRuns) {
    HttpHarness h;
    h.raise("bump");
    h.raise("go");

    ASSERT_EQ(h.posted.size(), 1u) << "one request is handed to the transport";
    const auto &request = h.posted[0];
    EXPECT_EQ(request.target, "http://example.invalid/hook");
    EXPECT_EQ(request.eventName, "note");
    EXPECT_EQ(request.content, "") << "no <content>, so the body is the pairs";
    const HttpParams wanted = {{"count", {"4"}}, {"ready", {"true"}}, {"label", {"busy"}},
                               {"twice", {"8"}}, {"delta", {"-5"}},   {"ratio", {"1.5"}}};
    EXPECT_EQ(request.params, wanted) << "each value is the text it spells: an integer's digits, `true`, the "
                                         "string, a negative number, a real's String()";
}

TEST(AStaticDatamodelRunsGeneratedCppTest, TheHttpPairsAreTheFieldsAsTheyStandAndNotACopyFromStartUp) {
    HttpHarness h;
    h.raise("go");

    ASSERT_EQ(h.posted.size(), 1u);
    const HttpParams wanted = {{"count", {"3"}}, {"ready", {"false"}}, {"label", {"idle"}},
                               {"twice", {"6"}}, {"delta", {"-5"}},    {"ratio", {"1.5"}}};
    EXPECT_EQ(h.posted[0].params, wanted) << "without `bump` the fields hold their initial values, and the request "
                                             "carries those: it is read when the send runs";
}

TEST(AStaticDatamodelRunsGeneratedCppTest, AHttpParamThatCannotBeReadIsLeftOutAndTheRequestStillGoes) {
    HttpHarness h;
    h.raise("bump");
    h.raise("boom");

    ASSERT_EQ(h.posted.size(), 1u) << "the request goes with the pair that could be read";
    const HttpParams wanted = {{"count", {"4"}}};
    EXPECT_EQ(h.posted[0].params, wanted) << "`big` is `count * 2000000000`, which a 32-bit field cannot hold: its "
                                             "pair is left out, not carried as a zero";
    EXPECT_EQ(h.sm.errors(), 1u) << "§scxml-5.7.1: the failed pair is reported as error.execution, once";
}

// The `delayexpr` of a `<send>` is a string computed from the machine's fields when
// the send runs, and read as the CSS2 time it must be; the machine runs on a manual
// clock, which the scenario's `advance_ms` steps move on.
TEST(AStaticDatamodelRunsGeneratedCppTest, ASendsDelayIsComputedWhenItRuns) {
    using Machine = G::static_send_delay::static_send_delay;
    Driver<Machine> driver({
        {"wait", [](const Machine &m) { return json(m.wait()); }},
        {"beats", [](const Machine &m) { return json(m.beats()); }},
        {"refusals", [](const Machine &m) { return json(m.refusals()); }},
    });
    replay("static_send_delay", driver);
}

// The `sendidexpr` of a `<cancel>` is a string computed from the machine's fields
// when the cancel runs, the id of the delayed send it removes; the machine runs on
// a manual clock, which the scenario's `advance_ms` steps move on.
TEST(AStaticDatamodelRunsGeneratedCppTest, ACancelRemovesTheSendItsIdNames) {
    using Machine = G::static_cancel_expr::static_cancel_expr;
    Driver<Machine> driver({
        {"a_fired", [](const Machine &m) { return json(m.a_fired()); }},
        {"b_fired", [](const Machine &m) { return json(m.b_fired()); }},
        {"refusals", [](const Machine &m) { return json(m.refusals()); }},
    });
    replay("static_cancel_expr", driver);
}

// A native host action takes the machine's variables as typed arguments, each
// read when the call is made, and the host the scenario's `host_calls` are read
// from records them: one call per entry of `idle`, with the datamodel as it stood.
TEST(AStaticDatamodelRunsGeneratedCppTest, AHostActionTellsItsHostWhatTheDatamodelHeld) {
    namespace Hc = G::static_host_call;
    using Machine = Hc::static_host_call;

    struct Host : Hc::StaticHostCallActions {
        json calls = json::array();

        void showAttempts(uint32_t count, bool exhausted) override {
            calls.push_back({{"action", "showAttempts"}, {"args", json::array({count, exhausted})}});
        }

        /// The calls made since this was last called, and forgets them.
        json take() {
            json taken = std::move(calls);
            calls = json::array();
            return taken;
        }
    } host;

    Driver<Machine> driver({}, host, [&host] { return host.take(); });
    replay("static_host_call", driver);
}

namespace {

/// Whether `Actions` declares what its host does with duties.
template <typename Actions, typename = void> struct DeclaresRealization : std::false_type {};

template <typename Actions>
struct DeclaresRealization<Actions, std::void_t<decltype(std::declval<const Actions &>().realization())>>
    : std::true_type {};

/// The host of `static_host_duty`: it records what the machine asked of it, by the
/// name the document gives the action, and says what it does with the duties.
struct DutyHost : G::static_host_duty::StaticHostDutyActions {
    explicit DutyHost(::SCE::ActionRealization declared) : declared(declared) {}

    ::SCE::ActionRealization realization() const override {
        return declared;
    }

    void startKeepalive() override {
        call("start_keepalive");
    }

    void noteEngaged() override {
        call("note_engaged");
    }

    void stopKeepalive() override {
        call("stop_keepalive");
    }

    /// The calls made since this was last called, and forgets them.
    json take() {
        json taken = std::move(calls);
        calls = json::array();
        return taken;
    }

    ::SCE::ActionRealization declared;
    json calls = json::array();

private:
    void call(const std::string &action) {
        calls.push_back({{"action", action}, {"args", json::array()}});
    }
};

}  // namespace

// docs/adr/0038: a machine that states a duty takes a host that says what it does
// with duties, a machine that states none takes a host with no such member, and the
// opt-in for a simulated host is a type that is written out and not a flag that can
// be passed by accident.
TEST(AStaticDatamodelRunsGeneratedCppTest, OnlyAMachineThatStatesADutyAsksItsHostWhatItDoesWithDuties) {
    static_assert(DeclaresRealization<G::static_host_duty::StaticHostDutyActions>::value,
                  "a machine that states a duty declares the realization in its host interface");
    static_assert(!DeclaresRealization<G::static_host_call::StaticHostCallActions>::value,
                  "a machine of notices takes a host with no realization member");
    static_assert(std::is_constructible_v<G::static_host_duty::static_host_duty,
                                          G::static_host_duty::StaticHostDutyActions &, ::SCE::SimulatedHost>,
                  "the opt-in is the trailing constructor argument of a machine that states a duty");
    static_assert(!std::is_constructible_v<G::static_host_duty::static_host_duty,
                                           G::static_host_duty::StaticHostDutyActions &, bool>,
                  "a bool does not reach the opt-in: it has to be written as SimulatedHost::Accept");
    static_assert(!std::is_constructible_v<G::static_host_call::static_host_call,
                                           G::static_host_call::StaticHostCallActions &, ::SCE::SimulatedHost>,
                  "a machine of notices has no opt-in to give");
}

// A host that says it performs the duties is the host a machine that states one is
// built for: no opt-in is given, and the machine takes the duty on entry and gives
// it back on exit.
TEST(AStaticDatamodelRunsGeneratedCppTest, AHostThatPerformsTheDutiesIsAcceptedWithoutAnOptIn) {
    using Machine = G::static_host_duty::static_host_duty;
    DutyHost host(::SCE::ActionRealization::Performs);

    Machine machine(host);
    machine.initialize();
    EXPECT_EQ(host.take(), json::array()) << "`idle` calls nothing on the host";

    machine.raiseExternal("engage");
    machine.step();
    EXPECT_EQ(host.take(), json::parse(R"([{"action": "start_keepalive", "args": []},
                                           {"action": "note_engaged", "args": []}])"))
        << "entering `engaged` takes the duty and then gives the notice";

    machine.raiseExternal("release");
    machine.step();
    EXPECT_EQ(host.take(), json::parse(R"([{"action": "stop_keepalive", "args": []}])"))
        << "leaving `engaged` gives the duty back through its <onexit>";
}

// A host that says it only simulates the duties is refused when the machine is
// built, before it is stored and before the first entry could call it.
TEST(AStaticDatamodelRunsGeneratedCppTest, AHostThatOnlySimulatesTheDutiesIsRefusedAtConstruction) {
    using Machine = G::static_host_duty::static_host_duty;
    DutyHost host(::SCE::ActionRealization::Simulates);

    EXPECT_THROW({ Machine machine(host); }, std::invalid_argument);

    std::string refusal;
    try {
        Machine machine(host);
    } catch (const std::invalid_argument &refused) {
        refusal = refused.what();
    }
    EXPECT_NE(refusal.find("static_host_duty"), std::string::npos) << "the message names the machine: " << refusal;
    EXPECT_NE(refusal.find("only simulates"), std::string::npos)
        << "the message says the host declares that it only simulates the duties: " << refusal;
    EXPECT_NE(refusal.find("SimulatedHost::Accept"), std::string::npos) << "the message names the opt-in: " << refusal;
    EXPECT_EQ(host.take(), json::array()) << "a refused host was not called";
}

// The caller that built the host for a recorder says so by name, and the machine
// runs its scenario against it: the Driver builds the machine through the opt-in.
TEST(AStaticDatamodelRunsGeneratedCppTest, ASimulatingHostIsAcceptedWhenTheCallerAcceptsOneByName) {
    using Machine = G::static_host_duty::static_host_duty;
    DutyHost host(::SCE::ActionRealization::Simulates);

    Driver<Machine> driver({}, host, [&host] { return host.take(); }, ::SCE::SimulatedHost::Accept);
    replay("static_host_duty", driver);
}

// The opt-in covers the child the parent builds: the parent's host answers the host
// of a child that states a duty, and the child is built with the opt-in the caller
// gave the parent. Without it the child refuses its host at its own construction.
TEST(AStaticDatamodelRunsGeneratedCppTest, TheOptInOfAParentCoversTheChildItBuildsWithItsHost) {
    namespace Pc = G::static_host_duty_child;
    namespace Wk = G::static_host_duty_child__sce_synth_invoke__worker;
    using Machine = Pc::static_host_duty_child;

    struct WorkerHost : Wk::StaticHostDutyChildSceSynthInvokeWorkerActions {
        explicit WorkerHost(::SCE::ActionRealization declared) : declared(declared) {}

        ::SCE::ActionRealization realization() const override {
            return declared;
        }

        void beginWork() override {
            calls.push_back("begin_work");
        }

        void endWork() override {
            calls.push_back("end_work");
        }

        ::SCE::ActionRealization declared;
        std::vector<std::string> calls;
    };

    struct Host : Pc::StaticHostDutyChildActions {
        explicit Host(::SCE::ActionRealization workers) : workers(workers) {}

        Wk::StaticHostDutyChildSceSynthInvokeWorkerActions &actionsForWorker() override {
            return started.emplace_back(workers);
        }

        ::SCE::ActionRealization workers;
        std::deque<WorkerHost> started;
    };

    const auto start = [](Machine &machine) {
        machine.setClock(std::make_shared<SCE::ManualClock>(0));
        machine.initialize();
        settleRounds(machine);
    };

    // The parent states no duty, so its own host has no realization to declare.
    static_assert(!DeclaresRealization<Pc::StaticHostDutyChildActions>::value);
    static_assert(DeclaresRealization<Wk::StaticHostDutyChildSceSynthInvokeWorkerActions>::value);

    {
        Host host(::SCE::ActionRealization::Performs);
        Machine machine(host);
        start(machine);
        ASSERT_EQ(host.started.size(), 1u) << "the parent's host was asked for the child's host once";
        EXPECT_EQ(host.started.front().calls, (std::vector<std::string>{"begin_work"}))
            << "the child took its duty on entry from a host that performs it, with no opt-in given";
    }
    {
        Host host(::SCE::ActionRealization::Simulates);
        Machine machine(host);
        // The child refuses a host that only simulates its duty, and the parent's caller
        // gave no opt-in. The refusal is the child's constructor throwing
        // `std::invalid_argument`, and the runtime's pending-invoke loop
        // (`InvokeHelper::executePendingInvokes`) catches a failed start, logs it and goes
        // on with the macrostep, so it does not reach the caller: what the test can hold
        // is that the child never performed a duty on the host that only simulates.
        EXPECT_NO_THROW(start(machine));
        ASSERT_EQ(host.started.size(), 1u) << "the parent's host was asked for the child's host";
        EXPECT_TRUE(host.started.front().calls.empty()) << "the refused host was not called";
    }
    {
        Host host(::SCE::ActionRealization::Simulates);
        Machine machine(host, ::SCE::SimulatedHost::Accept);
        start(machine);
        ASSERT_EQ(host.started.size(), 1u);
        EXPECT_EQ(host.started.front().calls, (std::vector<std::string>{"begin_work"}))
            << "the opt-in the caller gave the parent covers the child the parent builds";
    }
}

// A child that declares `<sce:action>`s is built with the host its parent's host
// answers for it, each time the invocation starts, and what the child asked of that
// host is what the scenario's `child_hosts` holds. The parent declares no act of its
// own: its host answers a host for each child and writes it into the book the
// driver reads. The parent's host owns what it answers, as a machine's own host is
// held by reference, so the children's hosts are kept in a deque.
TEST(AStaticDatamodelRunsGeneratedCppTest, EachStartOfAChildIsGivenAHostOfItsOwn) {
    namespace Ch = G::static_child_host;
    namespace Wk = G::static_child_host__sce_synth_invoke__worker;
    using Machine = Ch::static_child_host;

    struct WorkerHost : Wk::StaticChildHostSceSynthInvokeWorkerActions {
        explicit WorkerHost(ChildHostBook::Child &child) : child(child) {}

        void started() override {
            child.call("started");
        }

        void finished(uint32_t steps) override {
            child.call("finished", json::array({steps}));
        }

        ChildHostBook::Child &child;
    };

    struct Host : Ch::StaticChildHostActions {
        ChildHostBook book;
        std::deque<WorkerHost> workers;

        Wk::StaticChildHostSceSynthInvokeWorkerActions &actionsForWorker() override {
            return workers.emplace_back(book.open("worker"));
        }
    } host;

    Driver<Machine> driver(
        {
            {"completed", [](const Machine &m) { return json(m.completed()); }},
        },
        host, host.book);
    replay("static_child_host", driver);
}

// A hybrid `<invoke>` is given the host answered for the candidate it starts, and for
// no other.
TEST(AStaticDatamodelRunsGeneratedCppTest, ACandidateIsGivenTheHostAnsweredForIt) {
    namespace Hy = G::static_child_host_hybrid;
    using Machine = Hy::static_child_host_hybrid;

    struct FirstHost : G::static_hosted_first::StaticHostedFirstActions {
        explicit FirstHost(ChildHostBook::Child &child) : child(child) {}

        void firstRan() override {
            child.call("first_ran");
        }

        ChildHostBook::Child &child;
    };

    struct SecondHost : G::static_hosted_second::StaticHostedSecondActions {
        explicit SecondHost(ChildHostBook::Child &child) : child(child) {}

        void secondRan() override {
            child.call("second_ran");
        }

        ChildHostBook::Child &child;
    };

    struct Host : Hy::StaticChildHostHybridActions {
        ChildHostBook book;
        std::deque<FirstHost> firsts;
        std::deque<SecondHost> seconds;

        G::static_hosted_first::StaticHostedFirstActions &actionsForWork0jStaticHostedFirst() override {
            return firsts.emplace_back(book.open("work", "static_hosted_first"));
        }

        G::static_hosted_second::StaticHostedSecondActions &actionsForWork0jStaticHostedSecond() override {
            return seconds.emplace_back(book.open("work", "static_hosted_second"));
        }
    } host;

    Driver<Machine> driver(
        {
            {"completed", [](const Machine &m) { return json(m.completed()); }},
        },
        host, host.book);
    replay("static_child_host_hybrid", driver);
}

// A child session that talks to a Mesh peer is given a router of its own by the
// factory the replay registers as the machine's host (SCE_MESH.md, Child Sessions
// in Mesh): what it sends is recorded under its invoke and start, and a step's
// `invoke` reaches it through that router, a grandchild's through the router its
// parent's router answered for it.
TEST(AStaticDatamodelRunsGeneratedCppTest, EachChildIsGivenAMeshRouterOfItsOwn) {
    using Machine = G::static_child_mesh::static_child_mesh;
    Driver<Machine> driver({
        {"refusals", [](const Machine &m) { return json(m.refusals()); }},
    });
    replay("static_child_mesh", driver);
}

// A child given no router has its peer send refused as `error.execution` in the
// child, and the machine's own router is not asked: nothing falls back to it.
TEST(AStaticDatamodelRunsGeneratedCppTest, AChildGivenNoMeshRouterHasItsSendRefused) {
    using Machine = G::static_child_mesh::static_child_mesh;
    Machine machine;
    int parentSends = 0;
    machine.setClock(std::make_shared<SCE::ManualClock>(0));
    machine.setMeshSendCallback([&parentSends](const std::string &, const std::string &, const std::string &,
                                               const std::string &, const std::string &) {
        ++parentSends;
        return true;
    });
    machine.initialize();
    settleRounds(machine);
    EXPECT_EQ(machine.refusals(), 1u) << "the child ended on its refused greeting, and the machine counted it";
    EXPECT_EQ(parentSends, 0) << "a child's send fell back to the machine's router";
}

// A child session an `<invoke>` started is driven through its parent by
// autoforward, takes the events it waits for in order, and its end reaches the
// parent as `done.invoke`, which the parent counts.
TEST(AStaticDatamodelRunsGeneratedCppTest, AnInvokedChildIsDrivenAndCounted) {
    using Machine = G::static_invoke::static_invoke;
    Driver<Machine> driver({
        {"completed", [](const Machine &m) { return json(m.completed()); }},
    });
    replay("static_invoke", driver);
}

// An `<invoke>` hands its child the values its `<param>`s and `namelist` name, as
// they stand when the invoke executes, after the entry actions, and once.
TEST(AStaticDatamodelRunsGeneratedCppTest, AnInvokeHandsItsChildItsValuesOnce) {
    using Machine = G::static_invoke_params::static_invoke_params;
    Driver<Machine> driver({
        {"completed", [](const Machine &m) { return json(m.completed()); }},
    });
    replay("static_invoke_params", driver);
}

// A string an `<invoke>` hands its child is held to the bound the child declared,
// in bytes: a value past it is left out and raises `error.execution`, and the
// child still starts.
TEST(AStaticDatamodelRunsGeneratedCppTest, AStringHandedToAChildIsHeldToItsBoundInBytes) {
    using Machine = G::static_invoke_string::static_invoke_string;
    Driver<Machine> driver({
        {"completed", [](const Machine &m) { return json(m.completed()); }},
        {"errors", [](const Machine &m) { return json(m.errors()); }},
    });
    replay("static_invoke_string", driver);
}

// Leaving the state that holds an `<invoke>` cancels the child, which then ends
// nothing and counts nothing.
TEST(AStaticDatamodelRunsGeneratedCppTest, LeavingTheStateOfAnInvokeCancelsTheChild) {
    using Machine = G::static_invoke::static_invoke;
    Driver<Machine> driver({
        {"completed", [](const Machine &m) { return json(m.completed()); }},
    });
    replay("static_invoke_abort", driver);
}

// A `<history>` remembers what its parent held when it was left, and entering it
// brings that back: the shallow one the child that was active, the deep one the
// atomic states below, and a deep one of a `<parallel>` both regions at once. The
// scenario states the whole active configuration of each step.
TEST(AStaticDatamodelRunsGeneratedCppTest, AHistoryBringsBackWhatItsParentHeld) {
    using Machine = G::static_history::static_history;
    Driver<Machine> driver({
        {"resumed", [](const Machine &m) { return json(m.resumed()); }},
    });
    replay("static_history", driver);
}

// Four delayed sends armed on entering a state are delivered when each is due, two
// due the same moment in the order they were sent, and the last takes the machine
// to its final state; the machine runs on a manual clock, which the scenario's
// `advance_ms` steps move on.
TEST(AStaticDatamodelRunsGeneratedCppTest, TimersDeliverEachSendWhenItIsDue) {
    using Machine = G::static_timers::static_timers;
    Driver<Machine> driver({
        {"trace", [](const Machine &m) { return json(m.trace()); }},
    });
    replay("static_timers", driver);
}

// The same machine stopped: a `<cancel>` by the id of the longest send removes that
// one and no other, so the machine never ends.
TEST(AStaticDatamodelRunsGeneratedCppTest, StoppingCancelsTheSendItsIdNamesAndNoOther) {
    using Machine = G::static_timers::static_timers;
    Driver<Machine> driver({
        {"trace", [](const Machine &m) { return json(m.trace()); }},
    });
    replay("static_timers_stop", driver);
}

// The `idlocation` of a `<send>` names a string variable the machine writes the id
// it generates for the send to, which a later `<cancel sendidexpr>` names; the
// machine runs on a manual clock, which the scenario's `advance_ms` steps move on.
TEST(AStaticDatamodelRunsGeneratedCppTest, ASendHandsTheDocumentAnIdACancelCanName) {
    using Machine = G::static_send_idlocation::static_send_idlocation;
    Driver<Machine> driver({
        {"first_fired", [](const Machine &m) { return json(m.first_fired()); }},
        {"second_fired", [](const Machine &m) { return json(m.second_fired()); }},
        {"refusals", [](const Machine &m) { return json(m.refusals()); }},
    });
    replay("static_send_idlocation", driver);
}

// The `<content expr>` of a `<send>` names a record, which crosses as the pairs of
// its fields: a record variable and the payload of the event the transition is
// on, taken whole.
TEST(AStaticDatamodelRunsGeneratedCppTest, ASendCarriesTheRecordItsContentNames) {
    namespace E = G::static_send_content;
    using Machine = E::static_send_content;
    Driver<Machine> driver({
        {"shown",
         [](const Machine &m) {
             return json{{"layout", std::string(E::sceLogName(m.shown().layout))}, {"zoom", m.shown().zoom}};
         }},
        {"received", [](const Machine &m) { return json(std::string(E::sceLogName(m.received()))); }},
        {"level", [](const Machine &m) { return json(m.level()); }},
        {"relays", [](const Machine &m) { return json(m.relays()); }},
    });
    replay("static_send_content", driver);
}

// The `namelist` of a `<send>` names variables the machine holds, each carried as
// the pair `<param name="x" expr="x"/>` it abbreviates, an enum value among them
// as the name its enum declares.
TEST(AStaticDatamodelRunsGeneratedCppTest, ASendCarriesTheVariablesItsNamelistNames) {
    namespace E = G::static_send_namelist;
    using Machine = E::static_send_namelist;
    Driver<Machine> driver({
        {"layout", [](const Machine &m) { return json(std::string(E::sceLogName(m.layout()))); }},
        {"zoom", [](const Machine &m) { return json(m.zoom()); }},
        {"received", [](const Machine &m) { return json(std::string(E::sceLogName(m.received()))); }},
        {"level", [](const Machine &m) { return json(m.level()); }},
        {"deliveries", [](const Machine &m) { return json(m.deliveries()); }},
    });
    replay("static_send_namelist", driver);
}

// A `<send>` hands its event the pairs of its `<param>`s, each computed from the
// machine's own fields when the send runs. A pair whose value does not fit is
// reported and left out, the message still goes, and the receiver, finding the
// field missing, raises an `error.execution` of its own and takes no transition.
TEST(AStaticDatamodelRunsGeneratedCppTest, ASendHandsItsEventThePairsOfItsParams) {
    using Machine = G::static_send_params::static_send_params;
    Driver<Machine> driver({
        {"total", [](const Machine &m) { return json(m.total()); }},
        {"ok", [](const Machine &m) { return json(m.ok()); }},
        {"tag", [](const Machine &m) { return json(m.tag()); }},
        {"partialTotal", [](const Machine &m) { return json(m.partialTotal()); }},
        {"refusals", [](const Machine &m) { return json(m.refusals()); }},
    });
    replay("static_send_params", driver);
}

// A string variable is held to the UTF-8 bytes it declares: an assignment past the
// bound writes nothing, raises `error.execution` and ends its block, and the bound
// is a count of bytes — a `std::string` is made of them — not of characters.
TEST(AStaticDatamodelRunsGeneratedCppTest, AStringIsHeldToItsBytes) {
    using Machine = G::static_string_capacity::static_string_capacity;
    Driver<Machine> driver({
        {"title", [](const Machine &m) { return json(m.title()); }},
        {"body", [](const Machine &m) { return json(m.body()); }},
        {"copied", [](const Machine &m) { return json(m.copied()); }},
        {"refusals", [](const Machine &m) { return json(m.refusals()); }},
    });
    replay("static_string_capacity", driver);
}

// A bytes variable is held to the bytes it declares, as a string is to its UTF-8
// bytes: an assignment past the bound writes nothing, raises `error.execution` and
// ends its block. A scenario states a byte string as its byte-exact Latin-1 text.
TEST(AStaticDatamodelRunsGeneratedCppTest, AByteStringIsHeldToItsBound) {
    using Machine = G::static_bytes::static_bytes;
    Driver<Machine> driver({
        {"frame", [](const Machine &m) { return latin1Text(m.frame()); }},
        {"tail", [](const Machine &m) { return latin1Text(m.tail()); }},
        {"size", [](const Machine &m) { return json(m.size()); }},
        {"matches", [](const Machine &m) { return json(m.matches()); }},
        {"misses", [](const Machine &m) { return json(m.misses()); }},
        {"errors", [](const Machine &m) { return json(m.errors()); }},
    });
    replay("static_bytes", driver);
}

// A host is handed a byte string as a constant reference: the vector is the
// machine's own, and a write into it would change the variable behind its bound.
// Checked where it is compiled, since a reader that returned a copy or a mutable
// reference would be another type.
static_assert(std::is_same_v<decltype(std::declval<const G::static_bytes::static_bytes &>().frame()),
                             const std::vector<std::uint8_t> &>,
              "a published byte string is lent as a constant reference");

// An `<invoke type="scxml">` hands its child the values its `<param>`s and
// `namelist` name (§scxml-6.4.1), each to the child's variable of the same
// name. `worker` ends the moment it holds `start = 7` (a `<param>` reading
// `base`, which is 4 when `working` is entered and 7 once its entry action ran:
// the value is read when the invoke executes, at the end of the macrostep) and
// `enabled = true` (the `namelist`); handed less, it would wait and the parent
// would stay in `working`. `control`, the same child handed nothing, keeps what
// its `<data>` gave it and never ends.
class AStaticChildIsHandedItsParams : public ::testing::Test {
protected:
    using Machine = G::static_invoke_params::static_invoke_params;

    /// Let the child run and report to its parent.
    void settle() {
        for (int i = 0; i < 5; ++i) {
            machine.tick();
        }
    }

    std::string state() const {
        return Machine::PolicyType::getStateName(machine.getCurrentState());
    }

    void SetUp() override {
        machine.initialize();
        settle();
    }

    Machine machine;
};

TEST_F(AStaticChildIsHandedItsParams, AChildEndsOnceItHoldsTheValuesItsInvokeNames) {
    // `worker` ended, so it held both values: the parent left `working` and
    // counted it.
    EXPECT_EQ(state(), "plain");
    EXPECT_EQ(machine.completed(), 1u);
}

TEST_F(AStaticChildIsHandedItsParams, AChildHandedNothingKeepsTheValuesItsDataGaveIt) {
    settle();
    // `control` still waits for 7 and true, so `done.invoke.control` never
    // counted.
    EXPECT_EQ(state(), "plain");
    EXPECT_EQ(machine.completed(), 1u);
}

// A string an `<invoke type="scxml">` hands its child is held to the bound the
// child declared for that variable, in UTF-8 bytes, as an `<assign>` to it would
// be: a value past it is left out and reported (§scxml-5.7.1), and the child
// still starts, holding the one its `<data>` gave it. `fits` is handed the four
// bytes its `title` holds and ends on them; `over` is handed eight bytes and
// `wide` two characters of five bytes, so each starts with the 'ab' it ends on:
// 1 + 10 + 100, and two errors.
TEST(AStaticDatamodelRunsGeneratedCppTest, AStringHandedToAChildIsHeldToTheChildsBound) {
    G::static_invoke_string::static_invoke_string machine;
    machine.initialize();
    for (int i = 0; i < 5; ++i) {
        machine.tick();
    }
    EXPECT_EQ(machine.completed(), 111u);
    EXPECT_EQ(machine.errors(), 2u);
}

// An `<invoke srcexpr>` that declares `sce:candidates` starts the document its
// value names (§scxml-6.4), by the document's stem, and hands it the invoke's
// arguments, each to the variable of the same name that candidate declares
// (§scxml-6.4.3). The fixture runs four phases: `first` (a `file:` value),
// `second` (an absolute path), `lossy` (an `extra` no 32-bit field can hold:
// reported, left out, the child still ends) and `missing` (a document the invoke
// did not declare: reported, nothing starts). Each `done.invoke` adds a power of
// ten of its own, so 111 says which candidates ended, and the two errors are the
// `lossy` argument and the `missing` document. A candidate handed what the OTHER
// declares would never end.
class AStaticHybridInvokeStartsTheCandidateItsValueNames : public ::testing::Test {
protected:
    using Machine = G::static_invoke_hybrid::static_invoke_hybrid;

    void SetUp() override {
        machine.initialize();
        for (int i = 0; i < 40; ++i) {
            machine.tick();
        }
    }

    Machine machine;
};

TEST_F(AStaticHybridInvokeStartsTheCandidateItsValueNames, EachPhaseStartsTheCandidateItsValueNamesAndEnds) {
    EXPECT_EQ(machine.completed(), 111u);
    EXPECT_TRUE(machine.isInFinalState()) << "the last phase named no declared candidate, so the run is over";
}

TEST_F(AStaticHybridInvokeStartsTheCandidateItsValueNames, AnArgumentIsEvaluatedWhateverTheCandidateKeeps) {
    EXPECT_EQ(machine.errors(), 2u);
}

TEST_F(AStaticHybridInvokeStartsTheCandidateItsValueNames, AValueNamingNoDeclaredCandidateStartsNothing) {
    // `done.invoke.missing_run` would add 1000: nothing started to send it.
    EXPECT_LT(machine.completed(), 1000u);
}

// A hybrid `<invoke>` of a document that never mentions `error.execution` has no
// such event in its enum, and what the invoke does where it would raise one
// (a value naming no declared candidate, an argument that cannot be computed) is
// to start nothing and say nothing, not to name a member the document never
// declared. `static_invoke_hybrid_saved` is such a document: the watcher it
// starts is handed 7 and waits for an 8 nothing gives it.
TEST(AStaticDatamodelRunsGeneratedCppTest, AHybridInvokeOfADocumentWithNoErrorEventStartsItsCandidate) {
    G::static_invoke_hybrid_saved::static_invoke_hybrid_saved machine;
    machine.initialize();
    for (int i = 0; i < 5; ++i) {
        machine.tick();
    }
    EXPECT_EQ(machine.completed(), 0u);
    EXPECT_FALSE(machine.isInFinalState());
}

// A guard calls an imported algorithm with the record's own fields, and the
// bound it answers is computed on every step, not folded to a constant.
TEST(AStaticDatamodelRunsGeneratedCppTest, AGuardCallsAnImportedAlgorithmWithARecordsFields) {
    using Machine = G::static_record::static_record;
    Driver<Machine> driver({
        {"shown", [](const Machine &m) { return dayJson(m.shown()); }},
        {"refusals", [](const Machine &m) { return json(m.refusals()); }},
    });
    replay("static_record", driver);
}

// A sync run composed of the standard sync rules: each rule an imported
// algorithm that can fail, called in a guard and in an assignment with the
// event's typed payload as its arguments.
TEST(AStaticDatamodelRunsGeneratedCppTest, ASyncRunIsComposedOfTheStandardAlgorithms) {
    using Machine = G::sync_client::sync_client;
    Driver<Machine> driver({
        {"byToken", [](const Machine &m) { return json(m.byToken()); }},
        {"fullListing", [](const Machine &m) { return json(m.fullListing()); }},
        {"outcome", [](const Machine &m) { return json(m.outcome()); }},
        {"retryAt", [](const Machine &m) { return json(m.retryAt()); }},
        {"deleted", [](const Machine &m) { return json(m.deleted()); }},
        {"uploaded", [](const Machine &m) { return json(m.uploaded()); }},
        {"discarded", [](const Machine &m) { return json(m.discarded()); }},
        {"pages", [](const Machine &m) { return json(m.pages()); }},
        {"refusals", [](const Machine &m) { return json(m.refusals()); }},
    });
    replay("sync_client", driver);
}

/// One copy of the shared to-do list as a host reads it. One machine, three
/// scenarios: a person's copy twice and the hub's.
Driver<ClockPeeking<G::todo_peer::todo_peer>> todoPeerDriver() {
    using Machine = ClockPeeking<G::todo_peer::todo_peer>;
    return Driver<Machine>({
        {"node", [](const Machine &m) { return wideText(m.node()); }},
        {"hub", [](const Machine &m) { return json(m.hub()); }},
        {"clock", [](const Machine &m) { return hlcJson(m.publishedClock()); }},
        {"adds", [](const Machine &m) { return orsetEntriesJson(m.adds()); }},
        {"tombstones", [](const Machine &m) { return orsetEntriesJson(m.tombstones()); }},
        {"values", [](const Machine &m) { return todoValuesJson(m.values()); }},
        {"digest", [](const Machine &m) { return merkleNodesJson(m.digest()); }},
        {"refusals", [](const Machine &m) { return json(m.refusals()); }},
        {"agenda", [](const Machine &m) { return occurrencesJson(m.agenda()); }},
        {"agendaNext", [](const Machine &m) { return json(m.agendaNext()); }},
        {"floor", [](const Machine &m) { return wideText(m.floor()); }},
        {"stale", [](const Machine &m) { return json(m.stale()); }},
        {"late", [](const Machine &m) { return json(m.late()); }},
        {"full", [](const Machine &m) { return json(m.full()); }},
        {"denied", [](const Machine &m) { return json(m.denied()); }},
    });
}

// A copy of a shared to-do list merges the writes it is given with the standard
// merge rules, keeping every value of a field that nobody has written over as an
// observed-remove set does, and sends a person's change to the hub. The scenario
// is what one copy was given through a run of three, and states what it sent its
// peers at each step: the copy of a person who edits...
TEST(AStaticDatamodelRunsGeneratedCppTest, ACopyOfASharedListHoldsAndSendsWhatAliceDid) {
    auto driver = todoPeerDriver();
    replay("todo_peer_alice", driver);
}

// ...the copy of the other person, who takes the first one's writes from the hub...
TEST(AStaticDatamodelRunsGeneratedCppTest, ACopyOfASharedListHoldsAndSendsWhatBobDid) {
    auto driver = todoPeerDriver();
    replay("todo_peer_bob", driver);
}

// ...and the hub, which passes each write on to every copy but the one it came from.
TEST(AStaticDatamodelRunsGeneratedCppTest, TheHubOfASharedListPassesEachWriteOn) {
    auto driver = todoPeerDriver();
    replay("todo_peer_server", driver);
}

/// One copy of the standard shared record list (sce:std/shared/record_list.scxml) as a
/// host reads it: what the to-do list publishes but its schedule, and what it told the
/// statechart that invoked it.
Driver<ClockPeeking<G::record_list::record_list>> recordListDriver() {
    using Machine = ClockPeeking<G::record_list::record_list>;
    return Driver<Machine>({
        {"node", [](const Machine &m) { return wideText(m.node()); }},
        {"hub", [](const Machine &m) { return json(m.hub()); }},
        {"clock", [](const Machine &m) { return hlcJson(m.publishedClock()); }},
        {"adds", [](const Machine &m) { return orsetEntriesJson(m.adds()); }},
        {"tombstones", [](const Machine &m) { return orsetEntriesJson(m.tombstones()); }},
        {"values", [](const Machine &m) { return todoValuesJson(m.values()); }},
        {"digest", [](const Machine &m) { return merkleNodesJson(m.digest()); }},
        {"refusals", [](const Machine &m) { return json(m.refusals()); }},
        {"floor", [](const Machine &m) { return wideText(m.floor()); }},
        {"stale", [](const Machine &m) { return json(m.stale()); }},
        {"late", [](const Machine &m) { return json(m.late()); }},
        {"full", [](const Machine &m) { return json(m.full()); }},
        {"denied", [](const Machine &m) { return json(m.denied()); }},
        {"announced", [](const Machine &m) { return json(m.announced()); }},
    });
}

// The standard shared record list, its scenarios' machine named by its library name: a
// person's copy, which sends its changes to the hub by `#hub`...
TEST(AStaticDatamodelRunsGeneratedCppTest, TheStandardRecordListIsAPersonsCopy) {
    auto driver = recordListDriver();
    replay("record_list_person", driver);
}

// ...the hub, which passes every change on to the people by `#people`...
TEST(AStaticDatamodelRunsGeneratedCppTest, TheHubOfTheStandardRecordListPassesEachChangeOn) {
    auto driver = recordListDriver();
    replay("record_list_hub", driver);
}

// ...and a list a host started, which tells no parent of the registers its changes
// move, and fails at nothing.
TEST(AStaticDatamodelRunsGeneratedCppTest, AStandardRecordListStartedByAHostTellsNoParent) {
    auto driver = recordListDriver();
    replay("record_list_alone", driver);
}

// A copy given no Mesh router (`"router": false`) keeps every change it makes: what
// it does to itself is queued before a send to a peer is refused (docs/adr/0046).
TEST(AStaticDatamodelRunsGeneratedCppTest, AStandardRecordListWithNoRouterKeepsItsChanges) {
    auto driver = recordListDriver();
    replay("record_list_offline", driver);
}

// A change is one unit: staged, stamped once, taken in whole, and sent as parts a copy
// holds until they are all there (docs/adr/0046).
TEST(AStaticDatamodelRunsGeneratedCppTest, AStandardRecordListTakesAChangeAsOneUnit) {
    auto driver = recordListDriver();
    replay("record_list_change", driver);
}

// A field is emptied and a row retired by writing over what the copy has seen, and a
// write it had not seen stays (docs/adr/0046).
TEST(AStaticDatamodelRunsGeneratedCppTest, AStandardRecordListClearsAFieldAndRetiresARow) {
    auto driver = recordListDriver();
    replay("record_list_retire", driver);
}

// A statechart that invokes the standard record list by its library name is told what
// each register a change moved holds: its count and its values in the order of their
// dots, a count of 0 when every value was written over, and nothing for a write the
// list already held.
TEST(AStaticDatamodelRunsGeneratedCppTest, AParentIsToldWhatEachRegisterOfItsListHolds) {
    namespace P = G::record_list_parent;
    using Machine = P::record_list_parent;
    Driver<Machine> driver({
        {"told", [](const Machine &m) { return json(m.told()); }},
        {"shown", [](const Machine &m) { return registerJson(m.shown()); }},
        {"others", [](const Machine &m) { return registerValuesJson(m.others()); }},
        // The last change the list refused: the command at fault, and the reason by the
        // name the enum record_refusal declares (docs/adr/0046).
        {"refused",
         [](const Machine &m) {
             const auto refused = m.refused();
             return json{{"id", refused.id},
                         {"field", refused.field},
                         {"kind", refused.kind},
                         {"reason", std::string(P::sceLogName(refused.reason))}};
         }},
        {"refusedCount", [](const Machine &m) { return json(m.refusedCount()); }},
    });
    replay("record_list_parent", driver);
}

/// One copy of the shared sheet as a host reads it. One machine, three
/// scenarios: a person's copy twice and the hub's.
Driver<ClockPeeking<G::sheet_peer::sheet_peer>> sheetPeerDriver() {
    using Machine = ClockPeeking<G::sheet_peer::sheet_peer>;
    return Driver<Machine>({
        {"node", [](const Machine &m) { return wideText(m.node()); }},
        {"hub", [](const Machine &m) { return json(m.hub()); }},
        {"clock", [](const Machine &m) { return hlcJson(m.publishedClock()); }},
        {"cells", [](const Machine &m) { return lwwCellsJson(m.cells()); }},
        {"rows", [](const Machine &m) { return rgaNodesJson(m.rows()); }},
        {"gone", [](const Machine &m) { return rgaGonesJson(m.gone()); }},
        {"cols", [](const Machine &m) { return rgaNodesJson(m.cols()); }},
        {"goneCols", [](const Machine &m) { return rgaGonesJson(m.goneCols()); }},
        {"shown", [](const Machine &m) { return sheetShownJson(m.shown()); }},
        {"lamport", [](const Machine &m) { return wideText(m.lamport()); }},
        {"orphans", [](const Machine &m) { return json(m.orphans()); }},
        {"refusals", [](const Machine &m) { return json(m.refusals()); }},
    });
}

// A copy of a shared sheet keeps its cells, rows and columns with the standard
// merge rules, handing them the lists it holds and taking each back whole, and
// sends a person's edit to the hub. The scenario is what one copy was given
// through a run of three, and states what it sent its peers at each step: the
// copy of a person who edits...
TEST(AStaticDatamodelRunsGeneratedCppTest, ACopyOfASharedSheetHoldsAndSendsWhatAliceDid) {
    auto driver = sheetPeerDriver();
    replay("sheet_peer_alice", driver);
}

// ...the copy of the other person...
TEST(AStaticDatamodelRunsGeneratedCppTest, ACopyOfASharedSheetHoldsAndSendsWhatBobDid) {
    auto driver = sheetPeerDriver();
    replay("sheet_peer_bob", driver);
}

// ...and the hub, which passes each write on to every copy but the one it came from.
TEST(AStaticDatamodelRunsGeneratedCppTest, TheHubOfASharedSheetPassesEachWriteOn) {
    auto driver = sheetPeerDriver();
    replay("sheet_peer_server", driver);
}

/// One copy of the list of who is online as a host reads it. One machine, three
/// scenarios: a client's copy twice and the hub's.
Driver<G::presence_peer::presence_peer> presencePeerDriver() {
    using Machine = G::presence_peer::presence_peer;
    return Driver<Machine>({
        {"node", [](const Machine &m) { return json(m.node()); }},
        {"hub", [](const Machine &m) { return json(m.hub()); }},
        {"ttl", [](const Machine &m) { return json(m.ttl()); }},
        {"lease", [](const Machine &m) { return json(m.lease()); }},
        {"online", [](const Machine &m) { return presenceEntriesJson(m.online()); }},
        {"serial", [](const Machine &m) { return json(m.serial()); }},
        {"lost", [](const Machine &m) { return json(m.lost()); }},
        {"refusals", [](const Machine &m) { return json(m.refusals()); }},
    });
}

// A copy of the list of who is online keeps each entry for a lease, let go by the
// machine's own delayed send unless a later heartbeat renewed it, and the hub tells
// every copy what it was told. The machine runs on a manual clock which the
// scenario's `advance_ms` steps move on, and the scenario states what the copy sent
// its peers at each step: a client's copy...
TEST(AStaticDatamodelRunsGeneratedCppTest, ACopyOfWhoIsOnlineHoldsAndSendsWhatAliceDid) {
    auto driver = presencePeerDriver();
    replay("presence_peer_alice", driver);
}

// ...the copy of the other client...
TEST(AStaticDatamodelRunsGeneratedCppTest, ACopyOfWhoIsOnlineHoldsAndSendsWhatBobDid) {
    auto driver = presencePeerDriver();
    replay("presence_peer_bob", driver);
}

// ...and the hub, which lets a client go when its heartbeats stop and tells every copy.
TEST(AStaticDatamodelRunsGeneratedCppTest, TheHubOfWhoIsOnlineLetsGoWhatGoesSilent) {
    auto driver = presencePeerDriver();
    replay("presence_peer_server", driver);
}

// A list of records is filled by name from a record variable or a loop's item,
// walked by a `<foreach>`, and a record is taken whole.
TEST(AStaticDatamodelRunsGeneratedCppTest, AListHoldsRecordsAndAForeachWalksThem) {
    using Machine = G::static_record_list::static_record_list;
    Driver<Machine> driver({
        {"days", [](const Machine &m) { return daysJson(m.days()); }},
        {"copies", [](const Machine &m) { return daysJson(m.copies()); }},
        {"draft", [](const Machine &m) { return dayJson(m.draft()); }},
        {"last", [](const Machine &m) { return dayJson(m.last()); }},
        {"total", [](const Machine &m) { return json(m.total()); }},
        {"errors", [](const Machine &m) { return json(m.errors()); }},
    });
    replay("static_record_list", driver);
}

// A field of the element a list of records is indexed at is read like any
// number; an index outside the list reads nothing and raises error.execution.
TEST(AStaticDatamodelRunsGeneratedCppTest, AFieldOfAnIndexedRecordIsRead) {
    using Machine = G::static_record_list_index::static_record_list_index;
    Driver<Machine> driver({
        {"cursor", [](const Machine &m) { return json(m.cursor()); }},
        {"day", [](const Machine &m) { return json(m.day()); }},
        {"year", [](const Machine &m) { return json(m.year()); }},
        {"sum", [](const Machine &m) { return json(m.sum()); }},
        {"ordered", [](const Machine &m) { return json(m.ordered()); }},
        {"refusals", [](const Machine &m) { return json(m.refusals()); }},
        {"picked", [](const Machine &m) { return dayJson(m.picked()); }},
    });
    replay("static_record_list_index", driver);
}

// A record may hold an enum: its field is read back by the name the enum
// document declares.
TEST(AStaticDatamodelRunsGeneratedCppTest, ARecordHoldsAnEnumField) {
    namespace E = G::static_record_enum;
    using Machine = E::static_record_enum;
    auto viewJson = [](const auto &view) {
        return json{{"layout", std::string(E::sceLogName(view.layout))}, {"zoom", view.zoom}};
    };
    Driver<Machine> driver({
        {"shown", [&](const Machine &m) { return viewJson(m.shown()); }},
        {"seen",
         [&](const Machine &m) {
             json listed = json::array();
             for (const auto &view : m.seen()) {
                 listed.push_back(viewJson(view));
             }
             return listed;
         }},
        {"weeks", [](const Machine &m) { return json(m.weeks()); }},
        {"flips", [](const Machine &m) { return json(m.flips()); }},
        {"leads", [](const Machine &m) { return json(m.leads()); }},
    });
    replay("static_record_enum", driver);
}

// A 64-bit integer crosses a `<send>`'s `<param>` as the text of its digits and is
// read back from it: the low digits of a value past 2^53 are the ones that were
// sent, and a number past 2^53 - 1 is refused where it is read.
TEST(AStaticDatamodelRunsGeneratedCppTest, A64BitIntegerCrossesAsItsDigits) {
    using Machine = G::static_wire_wide::static_wire_wide;
    Driver<Machine> driver({
        {"low", [](const Machine &m) { return json(m.low()); }},
        {"smallLow", [](const Machine &m) { return json(m.smallLow()); }},
        {"large", [](const Machine &m) { return json(m.large()); }},
        {"negative", [](const Machine &m) { return json(m.negative()); }},
        {"relays", [](const Machine &m) { return json(m.relays()); }},
        {"refusals", [](const Machine &m) { return json(m.refusals()); }},
    });
    replay("static_wire_wide", driver);
}

// The payload of an event is a record of its schema taken whole: it replaces a
// record variable in one assignment and is appended whole to a list, the enum
// field read back by the name the enum document declares.
TEST(AStaticDatamodelRunsGeneratedCppTest, ThePayloadOfAnEventIsTakenWholeAsARecord) {
    namespace E = G::static_whole_payload;
    using Machine = E::static_whole_payload;
    auto viewJson = [](const auto &view) {
        return json{{"layout", std::string(E::sceLogName(view.layout))}, {"zoom", view.zoom}};
    };
    Driver<Machine> driver({
        {"shown", [&](const Machine &m) { return viewJson(m.shown()); }},
        {"seen",
         [&](const Machine &m) {
             json listed = json::array();
             for (const auto &view : m.seen()) {
                 listed.push_back(viewJson(view));
             }
             return listed;
         }},
        {"updates", [](const Machine &m) { return json(m.updates()); }},
        {"agendas", [](const Machine &m) { return json(m.agendas()); }},
        {"others", [](const Machine &m) { return json(m.others()); }},
    });
    replay("static_whole_payload", driver);
}

// A delivery written against an earlier revision of a schema that grew lacks the
// fields a later revision added, and takes their defaults; a delivery that lacks a
// field of the first revision is refused, and a field the schema does not name is
// not read.
TEST(AStaticDatamodelRunsGeneratedCppTest, ADeliveryOfAnEarlierRevisionTakesTheDefaultsOfTheFieldsAddedSince) {
    namespace E = G::static_schema_revision;
    using Machine = E::static_schema_revision;
    Driver<Machine> driver({
        {"last",
         [](const Machine &m) {
             const auto &last = m.last();
             return json{
                 {"level", last.level}, {"layout", std::string(E::sceLogName(last.layout))}, {"urgent", last.urgent}};
         }},
        {"taken", [](const Machine &m) { return json(m.taken()); }},
        {"refusals", [](const Machine &m) { return json(m.refusals()); }},
    });
    replay("static_schema_revision", driver);
}

// An enum value as a `<param>` crosses as the name its enum declares for it: a
// variable, a field of a record variable and a conditional, sent and read back
// through the schema.
TEST(AStaticDatamodelRunsGeneratedCppTest, AnEnumValueCrossesAsTheNameItsEnumDeclares) {
    namespace E = G::static_wire_enum;
    using Machine = E::static_wire_enum;
    Driver<Machine> driver({
        {"layout", [](const Machine &m) { return json(std::string(E::sceLogName(m.layout()))); }},
        {"shown",
         [](const Machine &m) {
             return json{{"layout", std::string(E::sceLogName(m.shown().layout))}, {"zoom", m.shown().zoom}};
         }},
        {"received", [](const Machine &m) { return json(std::string(E::sceLogName(m.received()))); }},
        {"deliveries", [](const Machine &m) { return json(m.deliveries()); }},
    });
    replay("static_wire_enum", driver);
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

namespace {

/// What a host recorded of the calls a machine made, as `(name, value…)` text.
using Calls = std::vector<std::string>;

}  // namespace

// A host action takes the machine's variables as typed arguments, each read
// when the call is made: one call per entry of `idle`, with the datamodel as it
// stood.
TEST(AStaticDatamodelRunsGeneratedCppTest, AHostActionTakesTypedDatamodelArguments) {
    namespace Hc = G::static_host_call;

    struct Host : Hc::StaticHostCallActions {
        Calls calls;

        void showAttempts(uint32_t count, bool exhausted) override {
            calls.push_back(std::to_string(count) + (exhausted ? ",true" : ",false"));
        }
    } host;

    Hc::static_host_call machine(host);
    machine.initialize();
    for (int i = 0; i < 4; ++i) {
        machine.raiseExternal("retry");
        machine.step();
    }
    // The fourth retry finds `attempts < 3` false and re-enters nothing.
    EXPECT_EQ(host.calls, (Calls{"0,false", "1,false", "2,false", "3,true"}));
}

// An argument that cannot be computed is a failure, not a wrapped value: the
// host is not called, and `error.execution` is raised in the call's place.
TEST(AStaticDatamodelRunsGeneratedCppTest, AnArgumentThatOverflowsStopsTheCallAndRaisesAnError) {
    namespace Hc = G::static_host_call_arguments;

    struct Host : Hc::StaticHostCallArgumentsActions {
        Calls calls;

        void report(uint8_t next) override {
            calls.push_back(std::to_string(next));
        }
    } host;

    Hc::static_host_call_arguments machine(host);
    machine.initialize();

    machine.raiseExternal("fine");
    machine.step();
    EXPECT_EQ(host.calls, (Calls{"251"})) << "250 + 1 fits a uint8";
    EXPECT_EQ(machine.errors(), 0);

    machine.raiseExternal("overflow");
    machine.step();
    EXPECT_EQ(host.calls, (Calls{"251"})) << "250 + 10 does not fit, so the host is not called with 4";
    EXPECT_EQ(machine.errors(), 1) << "error.execution was raised and the machine saw it";
}

// SCE_MESH.md, Errors: a send to a peer from a machine whose host registered no Mesh
// router is a configuration fault of the host and raises error.execution, as it does
// on every other backend. It does not go to the sender's own queue: todo_peer takes
// what a peer sends it as a write to its own list, so an event that came back would
// be taken for the peer's, and the one the machine sent itself over `#_internal` is
// the only write it holds.
TEST(AStaticDatamodelRunsGeneratedCppTest, ASendToAMeshPeerWithNoRouterRegisteredIsErrorExecution) {
    G::todo_peer::todo_peer machine;
    machine.initialize();
    machine.raiseExternal("todo.join", R"({"node":"1","hub":false})");
    machine.raiseExternal("todo.edit", R"({"id":7,"field":1,"value":"0","text":"buy milk","now":"0"})");
    machine.step();
    EXPECT_EQ(machine.refusals(), 1u) << "the send to #server was refused, once, as error.execution";
}

}  // namespace Tests
}  // namespace SCE
