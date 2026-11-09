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

#include "common/SendHelper.h"
#include "events/EventDispatcherImpl.h"
#include "events/EventSchedulerImpl.h"
#include "events/EventTargetFactoryImpl.h"
#include "host/StandardHost.h"
#include "integration/StandardHostScenarioReplay.h"
#include "runtime/EventRaiserImpl.h"
#include "runtime/IChildMeshRouter.h"
#include "runtime/INativeActionHost.h"
#include "runtime/StateMachine.h"
#include "scripting/ScriptEngineProvider.h"

#include <algorithm>
#include <atomic>
#include <chrono>
#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <filesystem>
#include <fstream>
#include <functional>
#include <gtest/gtest.h>
#include <map>
#include <memory>
#include <mutex>
#include <nlohmann/json.hpp>
#include <set>
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
/// documents its `<invoke>`s may start (a child named by `src`, a candidate of a
/// hybrid one), written into `dir`, and the paths
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
///
/// A 64-bit integer is a Number in the Interpreter's data model, and the text
/// of its digits in a scenario written from a saved state (a JSON number past
/// 2^53 is not exact everywhere). So an integer the engine holds matches that
/// text of the same integer, wherever in a record or a list it stands;
/// everything else must be equal as written.
bool holds(const nlohmann::json &got, const nlohmann::json &want) {
    if (want.is_string() && got.is_number_integer()) {
        return got.dump() == want.get<std::string>();
    }
    if (want.is_array() && got.is_array()) {
        if (got.size() != want.size()) {
            return false;
        }
        for (std::size_t n = 0; n < want.size(); ++n) {
            if (!holds(got[n], want[n])) {
                return false;
            }
        }
        return true;
    }
    if (want.is_object() && got.is_object()) {
        if (got.size() != want.size()) {
            return false;
        }
        for (const auto &[name, value] : want.items()) {
            if (!got.contains(name) || !holds(got[name], value)) {
                return false;
            }
        }
        return true;
    }
    return got == want;
}

/// What a machine sent its Mesh peers (`<send target="#name">`), kept in the
/// order it was sent. Written where the send is delivered and read by the test,
/// under one lock.
class SentToPeers {
public:
    /// `child` holds the `invoke` and `start` of the child session that sent it,
    /// and is empty for the machine's own send.
    void record(const EventDescriptor &event, const nlohmann::json &child = nlohmann::json::object()) {
        nlohmann::json send = {{"target", event.target}, {"event", event.eventName}};
        // The data the receiving session would read as `_event.data`, as the JSON
        // value of its text; a send that carried none states no data.
        const std::string data = event.payload();
        if (!data.empty()) {
            send["data"] = nlohmann::json::parse(data, nullptr, false);
        }
        send.update(child);
        std::lock_guard<std::mutex> lock(mutex_);
        sent_.push_back(std::move(send));
    }

    /// The sends made since this was last called, oldest first, each as a
    /// scenario's `sent` writes it: `{"target": …, "event": …, "data": …}`.
    nlohmann::json take() {
        std::lock_guard<std::mutex> lock(mutex_);
        nlohmann::json taken = std::move(sent_);
        sent_ = nlohmann::json::array();
        return taken;
    }

    /// What `take` would answer now, left in place: for a step that waits for
    /// what a child running in an event loop of its own has yet to send.
    nlohmann::json peek() const {
        std::lock_guard<std::mutex> lock(mutex_);
        return sent_;
    }

private:
    mutable std::mutex mutex_;
    nlohmann::json sent_ = nlohmann::json::array();
};

/// The target of a send to a Mesh peer: it writes the send down and delivers it
/// to no one, because the peer is not there. What a scenario holds a step to is
/// that the machine asked for the send. A child's target writes down, beside the
/// send, the `invoke` and `start` of the child that made it.
class PeerTarget : public IEventTarget {
public:
    PeerTarget(std::shared_ptr<SentToPeers> sent, std::string peer, nlohmann::json child = nlohmann::json::object())
        : sent_(std::move(sent)), peer_(std::move(peer)), child_(std::move(child)) {}

    std::future<SendResult> send(const EventDescriptor &event) override {
        sent_->record(event, child_);
        std::promise<SendResult> accepted;
        accepted.set_value(SendResult::success(event.sendId));
        return accepted.get_future();
    }

    std::string getTargetType() const override {
        return "mesh";
    }

    bool canHandle(const std::string &targetUri) const override {
        return targetUri == peer_;
    }

    std::vector<std::string> validate() const override {
        return {};
    }

    std::string getDebugInfo() const override {
        return "PeerTarget(" + peer_ + ")";
    }

private:
    std::shared_ptr<SentToPeers> sent_;
    std::string peer_;
    nlohmann::json child_;
};

/// The routers a replay gives the child sessions of the machine it drives
/// (SCE_MESH.md, Child Sessions in Mesh; docs/adr/0043). A child that sends to a
/// Mesh peer is given a router of its own by its parent, which asks the factory its
/// host registered; a replay is the host, so it registers one (`factory`) whose
/// routers write down what each child sent, told by the path of invoke ids that
/// started it and how many children that path had started, and raise what a step's
/// `invoke` delivers on the child's own raiser. A router answers for the child's
/// own children in turn, so a grandchild is told by `<invoke>/<invoke>`. The
/// Interpreter's half of the Rust replay's `child_mesh::Children`.
class ChildRouters {
public:
    explicit ChildRouters(std::shared_ptr<SentToPeers> sent) : book_(std::make_shared<Book>()) {
        book_->sent = std::move(sent);
    }

    /// The factory a replay registers on the machine: every child is given a
    /// router, whichever invoke starts it and from whatever document.
    ChildMeshRouters factory() const {
        return [book = book_](const std::string &invokeId, const std::string &) { return start(book, invokeId); };
    }

    /// Raise `event`, with `data` as its `_event.data`, on the running child of
    /// `path` as a peer's event, on its external queue. False when no child of
    /// `path` is running, which a step that names one fails on.
    bool deliver(const std::string &path, const std::string &event, const std::string &data) const {
        std::shared_ptr<IEventRaiser> inbound;
        {
            std::lock_guard<std::mutex> lock(book_->mutex);
            const auto running = book_->running.find(path);
            if (running == book_->running.end()) {
                return false;
            }
            inbound = running->second.inbound;
        }
        return inbound && inbound->raiseExternalEvent(event, data);
    }

private:
    /// The session a path is running: the router it was given, and the raiser it
    /// was attached to.
    struct Running {
        const IChildMeshRouter *router = nullptr;
        std::shared_ptr<IEventRaiser> inbound;
    };

    /// The list every send is written into, the starts of each path, and the
    /// session each path is running, under one lock: a child runs in an event loop
    /// of its own.
    struct Book {
        std::mutex mutex;
        std::shared_ptr<SentToPeers> sent;
        std::map<std::string, int> starts;
        std::map<std::string, Running> running;
    };

    /// The router of one start of one child.
    class Recording : public IChildMeshRouter {
    public:
        Recording(std::shared_ptr<Book> book, std::string path, int number)
            : book_(std::move(book)), path_(std::move(path)), number_(number) {}

        std::shared_ptr<IEventTarget> createTarget(const std::string &targetUri) override {
            return std::make_shared<PeerTarget>(book_->sent, targetUri,
                                                nlohmann::json{{"invoke", path_}, {"start", number_}});
        }

        void attach(std::shared_ptr<IEventRaiser> inbound) override {
            std::lock_guard<std::mutex> lock(book_->mutex);
            const auto running = book_->running.find(path_);
            if (running != book_->running.end() && running->second.router == this) {
                running->second.inbound = std::move(inbound);
            }
        }

        /// The path is no longer running this session: a step that names it fails
        /// until the path starts a child again.
        void detach() override {
            std::lock_guard<std::mutex> lock(book_->mutex);
            const auto running = book_->running.find(path_);
            if (running != book_->running.end() && running->second.router == this) {
                book_->running.erase(running);
            }
        }

        std::shared_ptr<IChildMeshRouter> childRouter(const std::string &invokeId, const std::string &) override {
            return start(book_, path_ + "/" + invokeId);
        }

    private:
        std::shared_ptr<Book> book_;
        std::string path_;
        int number_;
    };

    /// A router for a new start of `path`, which is now the session the path is
    /// running.
    static std::shared_ptr<IChildMeshRouter> start(const std::shared_ptr<Book> &book, const std::string &path) {
        std::lock_guard<std::mutex> lock(book->mutex);
        const int number = ++book->starts[path];
        auto router = std::make_shared<Recording>(book, path, number);
        book->running[path] = Running{router.get(), nullptr};
        return router;
    }

    std::shared_ptr<Book> book_;
};

/// A host that performs every operation and records each call as the JSON of
/// its arguments. A child session performs its actions on a thread of its own
/// once it has been started, so the calls are read under the lock they are
/// written under.
class RecordingHost : public INativeActionHost {
public:
    /// A recorder performs nothing: a machine that states a duty is built for it
    /// through `setSimulatedNativeActionHost`, and a child it answers for inherits
    /// that (docs/adr/0038).
    ActionRealization realization() const override {
        return ActionRealization::Simulates;
    }

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
        std::lock_guard<std::mutex> lock(mutex_);
        calls_.push_back({name, values});
        return true;
    }

    std::vector<std::pair<std::string, nlohmann::json>> calls() const {
        std::lock_guard<std::mutex> lock(mutex_);
        return calls_;
    }

    /// The calls made since this was last called, oldest first, each as a
    /// scenario's `host_calls` writes it: `{"action": …, "args": […]}`, the name
    /// the document gives the action and its arguments in the order the document
    /// gives them.
    nlohmann::json take() {
        std::lock_guard<std::mutex> lock(mutex_);
        nlohmann::json taken = nlohmann::json::array();
        for (; taken_ < calls_.size(); ++taken_) {
            taken.push_back({{"action", calls_[taken_].first}, {"args", calls_[taken_].second}});
        }
        return taken;
    }

    /// A child the invocation started is given a host of its own, a recording one
    /// like this, and this keeps it, with the invoke that started the child, how many
    /// children that invoke had started, and the stem of the document the invoke
    /// loaded (empty for a child written inline), in the order they were asked for.
    std::shared_ptr<INativeActionHost> hostForChild(const std::string &invokeId, const std::string &document) override {
        const auto host = std::make_shared<RecordingHost>();
        std::lock_guard<std::mutex> lock(mutex_);
        int start = 1;
        for (const auto &child : children_) {
            if (child.invoke == invokeId) {
                ++start;
            }
        }
        children_.push_back({invokeId, start, document, host});
        return host;
    }

    /// The child hosts asked for or called since this was last taken, oldest asked
    /// first, each as a scenario's `child_hosts` writes it: the invoke, the start of
    /// that invoke, the candidate's document stem where the invoke names one, and
    /// the calls made in this step alone. A child asked for earlier that called
    /// nothing since has no entry. `consume` false reads them without taking them,
    /// for a step that waits for what a child running on a thread of its own has yet
    /// to do.
    nlohmann::json childHosts(bool consume) {
        std::lock_guard<std::mutex> lock(mutex_);
        nlohmann::json fresh = nlohmann::json::array();
        for (auto &child : children_) {
            const auto calls = child.host->calls();
            nlohmann::json heard = nlohmann::json::array();
            for (std::size_t n = child.taken; n < calls.size(); ++n) {
                heard.push_back({{"action", calls[n].first}, {"args", calls[n].second}});
            }
            if (child.announced && heard.empty()) {
                continue;
            }
            nlohmann::json entry = {{"invoke", child.invoke}, {"start", child.start}};
            if (!child.document.empty()) {
                entry["candidate"] = child.document;
            }
            entry["host_calls"] = std::move(heard);
            fresh.push_back(std::move(entry));
            if (consume) {
                child.taken = calls.size();
                child.announced = true;
            }
        }
        return fresh;
    }

private:
    struct Child {
        std::string invoke;
        int start;
        std::string document;
        std::shared_ptr<RecordingHost> host;
        std::size_t taken = 0;
        bool announced = false;
    };

    mutable std::mutex mutex_;
    std::vector<std::pair<std::string, nlohmann::json>> calls_;
    std::size_t taken_ = 0;
    std::vector<Child> children_;
};

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

    /// A scheduler that delivers a delayed send to its target when it falls due.
    /// In MANUAL mode the wall clock fires nothing and the logical clock is the
    /// host's to move (`EventSchedulerImpl::advanceTime`), which is the clock the
    /// generated backends' replays hold the same scenarios to.
    static std::shared_ptr<EventSchedulerImpl> makeScheduler(SchedulerMode mode) {
        auto scheduler = std::make_shared<EventSchedulerImpl>(
            [](const EventDescriptor &event, std::shared_ptr<IEventTarget> target, const std::string &) -> bool {
                try {
                    return target->send(event).get().isSuccess;
                } catch (...) {
                    return false;
                }
            });
        scheduler->setMode(mode);
        return scheduler;
    }

    /// What a `<send>` and an `<invoke>` need to run: a dispatcher. Without one
    /// the Interpreter raises error.execution for a `<send>`, as for a send with
    /// nowhere to go, and a child session has no way to answer its parent. The
    /// events a machine sends itself are queued, and run when the raiser is
    /// asked to; the raiser is returned for that. A replay hands in the list its
    /// machine's sends to Mesh peers are written to, and those are delivered to
    /// no one; without one the Interpreter has no router for a peer, and the send
    /// is refused as error.execution.
    ///
    /// The scheduler is the wall clock's unless the caller hands in its own: a
    /// child session runs on a thread of its own and a test of one waits for it
    /// in real time, while a replay that moves time by the step moves the
    /// scheduler's logical clock itself (`makeScheduler(SchedulerMode::MANUAL)`).
    std::shared_ptr<EventRaiserImpl> wire(StateMachine &machine, std::shared_ptr<SentToPeers> sentToPeers = nullptr,
                                          std::shared_ptr<EventSchedulerImpl> scheduler = nullptr) {
        if (!scheduler) {
            scheduler = makeScheduler(SchedulerMode::AUTOMATIC);
        }
        auto eventRaiser = std::make_shared<EventRaiserImpl>();
        eventRaiser->setScheduler(scheduler);
        eventRaiser->setImmediateMode(false);
        machine.setEventRaiser(eventRaiser);
        std::shared_ptr<IEventTargetFactory> targets = std::make_shared<EventTargetFactoryImpl>(eventRaiser);
        if (sentToPeers) {
            // The Interpreter has no Mesh transport of its own: the factory creates
            // a target for `#name` only through a router the host registers under
            // the Mesh scheme. The send is caught at the target and not at the
            // dispatcher, because the target is where a send is delivered: at once
            // for an immediate one, and when it is due for a delayed one.
            targets->registerTargetType(SendHelper::MESH_TARGET_SCHEME, [sentToPeers](const std::string &peer) {
                return std::make_shared<PeerTarget>(sentToPeers, peer);
            });
        }
        machine.setEventDispatcher(std::make_shared<EventDispatcherImpl>(scheduler, std::move(targets)));
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

    /// Replay `scenario` against `document`, the file of the machine the scenario
    /// names once lowered: the first of the set `sce-codegen lower --out-dir` wrote,
    /// which stands beside the documents its `<invoke>`s may start.
    void replay(const std::filesystem::path &document, const nlohmann::json &scenario) {
        const auto machine = std::make_shared<StateMachine>(*engine_);
        // The host a machine's `<sce:action>`s go to, which records them: what a
        // step's `host_calls` is read from. A machine with no action never asks it.
        // The recorder simulates, and a machine that states a duty
        // (`static_host_duty`) refuses such a host unless it is accepted by name, so
        // it is installed through the entry that says the double is meant
        // (docs/adr/0038).
        const auto host = std::make_shared<RecordingHost>();
        machine->setSimulatedNativeActionHost(host);
        // What the machine sends its Mesh peers, which a step's `sent` is read
        // from. Wired before the machine starts, because its first entry can
        // already send.
        const auto sentToPeers = std::make_shared<SentToPeers>();
        // A scenario that says `"router": false` is of a machine whose host attached
        // no link (docs/adr/0046, section 4): it is given no router for a peer and no
        // factory for its children, so every send to a peer is refused where it is
        // made.
        const bool routed = scenario.value("router", true);
        // A child session the machine starts is given a router of its own by the
        // factory its host registers (SCE_MESH.md, Child Sessions in Mesh), never
        // the machine's: what a child sends is written into the same list, told by
        // the invoke that started it and the start, and a step's `invoke` hands its
        // event to the child through it.
        const ChildRouters children(sentToPeers);
        if (routed) {
            machine->registerChildMeshRouters(children.factory());
        }
        // The time a step's `advance_ms` moves is this scheduler's logical clock,
        // not the wall: the scenario says how far time goes, and the delayed sends
        // that fall due in that span are released, with nothing waited out.
        const auto scheduler = makeScheduler(SchedulerMode::MANUAL);
        const auto eventRaiser = wire(*machine, routed ? sentToPeers : std::shared_ptr<SentToPeers>(), scheduler);
        ASSERT_TRUE(machine->loadSCXML(document.string())) << "the Interpreter does not load the lowered document";
        ASSERT_TRUE(machine->start());
        eventRaiser->processQueuedEvents();

        const auto &steps = scenario.at("steps");
        ASSERT_FALSE(steps.empty()) << "a scenario with no steps judges nothing";
        for (std::size_t n = 0; n < steps.size(); ++n) {
            const auto &step = steps[n];
            const std::string note = step.value("note", std::string{});
            SCOPED_TRACE("step " + std::to_string(n) + (note.empty() ? "" : " (" + note + ")"));
            // A step that moves the machine's time on, for a scenario of a delayed
            // send: the scheduler's logical clock moves on by exactly that much and
            // what fell due in the span is delivered, in the order of its deadlines.
            if (step.contains("advance_ms")) {
                scheduler->advanceTime(std::chrono::milliseconds(step.at("advance_ms").get<int64_t>()));
                eventRaiser->processQueuedEvents();
            }
            if (step.contains("event")) {
                const std::string data = step.contains("data") ? step.at("data").dump() : std::string{};
                const std::string event = step.at("event").get<std::string>();
                if (step.contains("invoke")) {
                    // A peer's event for the running child of that invoke, raised on
                    // the child through the router it was given.
                    const std::string path = step.at("invoke").get<std::string>();
                    EXPECT_TRUE(children.deliver(path, event, data))
                        << "no child of '" << path << "' is running to be sent '" << event << "'";
                } else {
                    machine->processEvent(event, data);
                }
                eventRaiser->processQueuedEvents();
            }
            const auto &expect = step.at("expect");
            // What the machine asked of its host in this step. Read on every step
            // so that a step which states nothing does not hand its calls to the
            // next.
            const auto asked = host->take();
            if (expect.contains("host_calls")) {
                EXPECT_EQ(asked, expect.at("host_calls")) << "what the machine asked of its host";
            }
            // The child hosts the machine's host was asked for in this step, and what
            // each child asked of its host, taken and held the same way. A child
            // session runs on a thread of its own, so what it did is there when the
            // step has been given time to be told, not when the event returns: a step
            // that states some waits until the hosts show it, and is judged on what
            // they show when that time is up.
            if (expect.contains("child_hosts")) {
                settles(eventRaiser, [&] { return host->childHosts(false) == expect.at("child_hosts"); });
            }
            const auto children = host->childHosts(true);
            if (expect.contains("child_hosts")) {
                EXPECT_EQ(children, expect.at("child_hosts"))
                    << "the child hosts the machine's host was asked for, and what their children asked of them";
            }
            // What the machine sent its Mesh peers in this step, taken on every
            // step for the same reason; compared as values, in the order sent,
            // with a 64-bit integer held to the text of its digits as a variable is.
            // A child session runs in an event loop of its own, so a step that
            // states what a child sent waits until the list shows it, and is judged
            // on what it shows when that time is up.
            if (expect.contains("sent") &&
                std::any_of(expect.at("sent").begin(), expect.at("sent").end(),
                            [](const nlohmann::json &want) { return want.contains("invoke"); })) {
                settles(eventRaiser, [&] { return holds(sentToPeers->peek(), expect.at("sent")); });
            }
            const auto sent = sentToPeers->take();
            if (expect.contains("sent")) {
                EXPECT_TRUE(holds(sent, expect.at("sent")))
                    << "what the machine sent its peers: " << sent.dump() << ", not " << expect.at("sent").dump();
            }
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
            // Every active state, a compound or a parallel one with the atomic
            // ones below it. A set: the order a machine lists them in is not part
            // of the answer.
            if (expect.contains("configuration")) {
                const auto active = machine->getActiveStates();
                EXPECT_EQ(expect.at("configuration").get<std::set<std::string>>(),
                          std::set<std::string>(active.begin(), active.end()))
                    << "the active states";
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
    std::vector<std::string> restoring;
    for (const auto &entry : std::filesystem::directory_iterator(kFixtures / "scenarios")) {
        if (entry.path().extension() != ".json") {
            continue;
        }
        const std::string name = entry.path().stem().string();
        SCOPED_TRACE(name);
        const auto scenario = nlohmann::json::parse(slurp(entry.path()), nullptr, false);
        ASSERT_FALSE(scenario.is_discarded()) << "a scenario is JSON";
        const std::string machine = scenario.at("machine").get<std::string>();

        // A step that restores the machine from a saved state (`restore`,
        // docs/adr/0045) needs an engine that saves, and the Interpreter does
        // not: such a scenario is skipped and counted, never passed.
        const auto &steps = scenario.at("steps");
        if (std::any_of(steps.begin(), steps.end(),
                        [](const nlohmann::json &step) { return step.contains("restore"); })) {
            restoring.push_back(name);
            continue;
        }

        // The lowering is asked for as a set, a directory: a machine whose `<invoke>`
        // starts a document of its own (by `src`, or a hybrid one's candidate) is
        // refused as one document, since what it starts stands beside it, and a
        // refusal that told the two apart by its text would pass a scenario of one
        // as not yet lowered. A machine with none is a set of one.
        const auto dir =
            std::filesystem::temp_directory_path() / ("sce-scenario-" + std::to_string(::getpid()) + "-" + name);
        std::error_code ec;
        std::filesystem::remove_all(dir, ec);
        // A fixture is named by its stem, and a statechart of the standard library by its
        // library name, which `sce-codegen lower` reads from the library it embeds.
        const bool standard = machine.rfind("sce:std/", 0) == 0;
        const std::filesystem::path document =
            standard ? std::filesystem::path(machine + ".scxml") : kFixtures / (machine + ".scxml");
        const LoweredSet set = lowerSet(document, dir);
        if (!set.ok) {
            // A refusal is an answer, and it must be the typed one: a crash
            // or a bare failure would read the same as "not yet lowered".
            ASSERT_TRUE(set.refusal.is_object()) << "`sce-codegen lower` failed with no diagnostic";
            EXPECT_EQ("generate/unsupported-feature", set.refusal.value("code", std::string{})) << set.refusal.dump();
            notYetLowered.push_back(name);
            std::filesystem::remove_all(dir, ec);
            continue;
        }
        replay(set.documents.at(0), scenario);
        replayed.push_back(name);
        std::filesystem::remove_all(dir, ec);
    }

    // Floor: a scan that found nothing to replay would pass, so what it must
    // have replayed is asserted. Every scenario is of a machine whose every
    // construct the lowering covers today, and the floor rises as the lowering
    // grows.
    EXPECT_GE(replayed.size(), 14u) << "replayed " << replayed.size()
                                    << " scenarios, not yet lowered: " << notYetLowered.size()
                                    << ", skipped for a restore: " << restoring.size();
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
    // ... and a payload of a schema that grew, whose fields added after the first
    // revision are read with the default a delivery that lacks them takes.
    EXPECT_NE(std::find(replayed.begin(), replayed.end(), "static_schema_revision"), replayed.end())
        << "static_schema_revision was not replayed: a schema's default has no lowering for the Interpreter";
    // ... and the machine that states a duty and a notice, replayed with a recorder
    // that simulates (docs/adr/0038): were it not accepted by name, `start()` would
    // refuse the document and the replay would fail before its first step.
    EXPECT_NE(std::find(replayed.begin(), replayed.end(), "static_host_duty"), replayed.end())
        << "static_host_duty was not replayed: a host duty has no lowering for the Interpreter";
    // ... and one carried by a `<param>` and a `<donedata>`, as its Latin-1 text.
    EXPECT_NE(std::find(replayed.begin(), replayed.end(), "static_bytes_wire"), replayed.end())
        << "static_bytes_wire was not replayed: a byte string as a param has no lowering for the Interpreter";
    // ... and the string literals whose escapes the lexer reads into their text, each
    // written again as the ECMAScript literal the script engine reads back as that text.
    EXPECT_NE(std::find(replayed.begin(), replayed.end(), "static_string_literal"), replayed.end())
        << "static_string_literal was not replayed: a string literal has no lowering for the Interpreter";
    // ... and the copies of a shared list and a shared sheet, composed of the standard
    // merge rules: each holds lists of records with 64-bit fields and sends to its
    // Mesh peers, which its scenario states step by step.
    for (const char *copy : {"todo_peer_alice", "todo_peer_bob", "todo_peer_carol", "sheet_peer_alice",
                             "sheet_peer_bob", "sheet_peer_server"}) {
        EXPECT_NE(std::find(replayed.begin(), replayed.end(), copy), replayed.end())
            << copy << " was not replayed: a copy that merges with the standard rules has no lowering for the "
            << "Interpreter";
    }
    // ... and the standard shared record list (sce:std/shared/record_list.scxml), a
    // scenario's machine named by its library name and lowered from the library: a
    // person's copy, a list a host started, when it tells no parent anything, and a
    // change and the one it writes over in their causal order, each change one message
    // whose operations are a list of records (docs/adr/0085) ...
    for (const char *copy : {"record_list_person", "record_list_alone", "record_list_offline", "record_list_change",
                             "record_list_retire", "record_list_out_of_causal_order_is_not_given"}) {
        EXPECT_NE(std::find(replayed.begin(), replayed.end(), copy), replayed.end())
            << copy << " was not replayed: the standard shared record list has no lowering for the Interpreter";
    }
    // ... and the statechart that invokes it by its library name: a child named by
    // `src` is lowered as a document of its own and written beside its parent, whose
    // `src` is rewritten to the name it is written under, so the Interpreter loads
    // the lowered list from the directory the set stands in.
    for (const char *invoking : {"record_list_parent", "record_list_concurrent_clears"}) {
        EXPECT_NE(std::find(replayed.begin(), replayed.end(), invoking), replayed.end())
            << invoking << " was not replayed: a child named by `src` has no lowering for the Interpreter";
    }
    // ... and the scenario of values that may be absent (docs/adr/0055), each `null` in a
    // variable, a record, a list, a payload, a send and the done data, so that a refusal
    // the scan above would let pass does not stand for its lowering.
    EXPECT_NE(std::find(replayed.begin(), replayed.end(), "static_optional"), replayed.end())
        << "static_optional was not replayed: an optional value has no lowering for the Interpreter";
    // ... and the algorithms handed values that may be absent: a list of records
    // whose day may be absent, and an optional day, each in and out.
    EXPECT_NE(std::find(replayed.begin(), replayed.end(), "static_state_algorithm_optional"), replayed.end())
        << "static_state_algorithm_optional was not replayed: an algorithm's optional value has no lowering for "
        << "the Interpreter";
    // ... and a record that may be absent (docs/adr/0055, step 9): taken whole from a payload, a
    // record variable and a list's element, read and written through `pick.value.<field>` where a
    // test proves it present, and answered by an algorithm, so that a refusal the scan above would
    // let pass does not stand for its lowering. The six generated backends refuse it by name until
    // each one's own step.
    EXPECT_NE(std::find(replayed.begin(), replayed.end(), "static_optional_record"), replayed.end())
        << "static_optional_record was not replayed: an optional record has no lowering for the Interpreter";
    // ... and a text read into a record or its refusal kept as one (docs/adr/0087, section
    // 9.1), through the library's own reader of JSON text, so that a refusal the scan
    // above would let pass does not stand for its lowering.
    EXPECT_NE(std::find(replayed.begin(), replayed.end(), "static_lift"), replayed.end())
        << "static_lift was not replayed: <sce:lift> has no lowering for the Interpreter";
    // ... and a text template rendered into a string variable (docs/adr/0087, section 3.1),
    // through the library's one `render` handed the template's parts, so that a refusal the
    // scan above would let pass does not stand for its lowering.
    EXPECT_NE(std::find(replayed.begin(), replayed.end(), "static_render"), replayed.end())
        << "static_render was not replayed: <sce:render> has no lowering for the Interpreter";
    // ... and the copies of the list of who is online, whose entries are let go by the
    // machine's own delayed send, which the Interpreter's scheduler releases as the
    // scenario moves its logical clock on.
    for (const char *copy : {"presence_peer_alice", "presence_peer_bob", "presence_peer_server"}) {
        EXPECT_NE(std::find(replayed.begin(), replayed.end(), copy), replayed.end())
            << copy << " was not replayed: a copy whose entries expire by a delayed send has no lowering for the "
            << "Interpreter";
    }
    // ... and a child that declares `<sce:action>`s, given the host its parent's host
    // answers for it: an inline one, and the candidate a hybrid `<invoke>` starts. A
    // machine with a hybrid invoke is lowered as a set and not as one document, so the
    // scan must not take that refusal for a lowering that does not exist yet.
    for (const char *parent : {"static_child_host", "static_child_host_hybrid"}) {
        EXPECT_NE(std::find(replayed.begin(), replayed.end(), parent), replayed.end())
            << parent << " was not replayed: a child given a host by its parent has no lowering for the Interpreter";
    }
    // ... and children that talk to Mesh peers, each given a router of its own by the
    // replay's factory (SCE_MESH.md, Child Sessions in Mesh): an inline one, and a
    // hybrid candidate whose own child is given the router the candidate's answers.
    EXPECT_NE(std::find(replayed.begin(), replayed.end(), "static_child_mesh"), replayed.end())
        << "static_child_mesh was not replayed: a child given a Mesh router by its parent has no lowering for the "
        << "Interpreter";
    for (const auto &name : notYetLowered) {
        RecordProperty("not_yet_lowered_" + name, "refused by sce-codegen lower");
    }
}

// docs/adr/0084, D7: the scenarios of the standard host features
// (tests/host_features/scenarios), replayed on the lowered documents of the machines
// the generated C++ engine runs them on, with the reference host registered on the
// Interpreter as it is registered there (`SCE::StandardHost::serve`), and each scenario
// read, its host set and judged by the replay the two engines share
// (integration/StandardHostScenarioReplay.h). A step's `advance_ms` moves the
// scheduler's logical clock, which the deadline of a host-run invocation is a moment
// of, as the generated machine's clock is moved there.
//
// A model's generation (`sce-host-model-generate`) is the one host-run type the lowering
// refuses (docs/adr/0087, section 7.1): the build writes its request's `answer_schema`
// and the machine's table of answer schemas, and the lowered document carries neither.
// Its scenario is answered by that refusal, by name, and replayed on the generated lanes; so are
// the five of the reference harness (docs/adr/0087, sections 6 and 7.1), whose host-run invoke is
// that generation.
TEST_F(AStaticDatamodelRunsLoweredUnderTheInterpreterTest, TheStandardHostServesTheInterpreterAsItServesGeneratedCode) {
    const auto names = HostScenario::scenarioNames();
    ASSERT_EQ(names,
              (std::vector<std::string>{"static_host_clock", "static_host_http", "static_host_kv", "static_host_model",
                                        "static_host_notify", "static_model_harness_accepted",
                                        "static_model_harness_exhausted", "static_model_harness_not_offered",
                                        "static_model_harness_repaired", "static_model_harness_timeout"}))
        << "every scenario of the directory is replayed or refused by name here";
    const std::set<std::string> refusedByName{"static_host_model", "static_model_harness"};
    for (const auto &name : names) {
        SCOPED_TRACE(name);
        const auto scenario = HostScenario::readScenario(name);
        const std::string machineName = scenario.at("machine").get<std::string>();
        const auto dir =
            std::filesystem::temp_directory_path() / ("sce-host-scenario-" + std::to_string(::getpid()) + "-" + name);
        std::error_code ec;
        std::filesystem::remove_all(dir, ec);
        const LoweredSet set = lowerSet(kFixtures / (machineName + ".scxml"), dir);
        if (refusedByName.count(machineName) != 0) {
            ASSERT_FALSE(set.ok) << "`sce-codegen lower` lowered " << machineName << ", whose generation it refuses";
            ASSERT_TRUE(set.refusal.is_object()) << "`sce-codegen lower` failed with no diagnostic";
            EXPECT_EQ("generate/unsupported-feature", set.refusal.value("code", std::string{})) << set.refusal.dump();
            EXPECT_NE(set.refusal.value("message", std::string{}).find("sce-host-model-generate"), std::string::npos)
                << "the refusal names the model's generation: " << set.refusal.dump();
            std::filesystem::remove_all(dir, ec);
            continue;
        }
        ASSERT_TRUE(set.ok) << "`sce-codegen lower` refused " << machineName << ": " << set.refusal.dump();

        // Declared before the machine, so it outlives every call the machine makes of it.
        HostScenario::Rig rig;
        const auto machine = std::make_shared<StateMachine>(*engine_);
        SCE::StandardHost::serve(*machine, rig.host);
        const auto scheduler = makeScheduler(SchedulerMode::MANUAL);
        const auto eventRaiser = wire(*machine, nullptr, scheduler);
        ASSERT_TRUE(machine->loadSCXML(set.documents.at(0).string()))
            << "the Interpreter does not load the lowered document";
        ASSERT_TRUE(machine->start());
        eventRaiser->processQueuedEvents();

        HostScenario::HostScenarioDriver driver;
        driver.advance = [&](uint64_t ms) {
            scheduler->advanceTime(std::chrono::milliseconds(static_cast<int64_t>(ms)));
            eventRaiser->processQueuedEvents();
        };
        driver.raise = [&](const std::string &, const std::string &event, const std::string &data) {
            machine->processEvent(event, data);
            eventRaiser->processQueuedEvents();
        };
        driver.state = [&]() { return machine->getCurrentState(); };
        driver.expectVariable = [&](const std::string &where, const std::string &variableName,
                                    const nlohmann::json &want) {
            const auto got = variable(*machine, variableName);
            EXPECT_TRUE(holds(got, want))
                << where << ": variable `" << variableName << "` is " << got.dump() << ", not " << want.dump();
        };
        HostScenario::replay(scenario, rig, driver);
        machine->stop();
        std::filesystem::remove_all(dir, ec);
    }
}

namespace {

/// A host that performs no operation of its own and answers each child its own
/// `RecordingHost`, keeping what it was asked and what it answered.
class ParentHost : public INativeActionHost {
public:
    ActionRealization realization() const override {
        return ActionRealization::Simulates;
    }

    bool performNativeAction(const std::string &, const std::vector<ScriptValue> &) override {
        return false;
    }

    std::shared_ptr<INativeActionHost> hostForChild(const std::string &invokeId, const std::string &document) override {
        std::lock_guard<std::mutex> lock(mutex_);
        asked_.emplace_back(invokeId, document);
        answered_.push_back(std::make_shared<RecordingHost>());
        return answered_.back();
    }

    /// Each question, as the invoke id and the document stem it named.
    std::vector<std::pair<std::string, std::string>> asked() const {
        std::lock_guard<std::mutex> lock(mutex_);
        return asked_;
    }

    /// Each host answered, in the order the questions came.
    std::vector<std::shared_ptr<RecordingHost>> answered() const {
        std::lock_guard<std::mutex> lock(mutex_);
        return answered_;
    }

private:
    mutable std::mutex mutex_;
    std::vector<std::pair<std::string, std::string>> asked_;
    std::vector<std::shared_ptr<RecordingHost>> answered_;
};

}  // namespace

// `static_host_call`'s host operation is the host's, so what the machine holds is
// the calls the host was given, which its scenario states as `host_calls` and
// every backend, this one included, is held to. This is the same machine driven by
// hand: the Interpreter performs the action through the host installed on the
// machine, with the arguments its engine computes from the lowered expressions —
// a variable, and a comparison over it.
TEST_F(AStaticDatamodelRunsLoweredUnderTheInterpreterTest, AHostActionIsPerformedWithTheValuesItsArgumentsComputeTo) {
    const Lowered lowered = lower(kFixtures / "static_host_call.scxml");
    ASSERT_TRUE(lowered.ok) << "`<sce:action>` is lowered for the Interpreter now: " << lowered.refusal.dump();

    const auto host = std::make_shared<RecordingHost>();
    const auto machine = std::make_shared<StateMachine>(*engine_);
    // `showAttempts` is a notice, which the machine does not depend on the host
    // performing, so a recorder is installed the plain way (docs/adr/0038).
    machine->setNativeActionHost(host);
    ASSERT_TRUE(machine->loadSCXMLFromString(lowered.document));
    ASSERT_TRUE(machine->start());
    for (int retry = 0; retry < 4; ++retry) {
        machine->processEvent("retry", "");
    }

    const auto calls = host->calls();
    ASSERT_EQ(calls.size(), 4u) << "one call per entry of `idle`; the fourth retry finds `attempts < 3` false";
    const std::vector<std::pair<int, bool>> expected = {{0, false}, {1, false}, {2, false}, {3, true}};
    for (std::size_t n = 0; n < expected.size(); ++n) {
        SCOPED_TRACE("call " + std::to_string(n));
        EXPECT_EQ(calls[n].first, "showAttempts");
        EXPECT_EQ(calls[n].second, nlohmann::json::array({expected[n].first, expected[n].second}))
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

// The door holds wherever an event's type is known, whatever the data model
// (docs/adr/0086, section 4.2, amended 2026-11-09): an `ecmascript` document whose
// guard reads a payload through its schema is lowered with the door of that event
// installed, and a delivery that does not fit is refused whole, with
// error.execution in its place, as every generated machine refuses it.
// `statechart_lifted` goes to `refused` on error.execution and to `done` on a
// `job.completed` whose `elapsed_ms`, a `uint32`, is 0. A text in its place compares
// false in the script engine and raises nothing, so only the door takes the
// machine to `refused`.
TEST_F(AStaticDatamodelRunsLoweredUnderTheInterpreterTest, AnEcmascriptDocumentRefusesADeliveryOutsideItsSchemaWhole) {
    const Lowered lowered = lower(std::filesystem::path(SCE_PROJECT_ROOT) /
                                  "sce-build/tests/fixtures/event_schema/statechart_lifted.scxml");
    ASSERT_TRUE(lowered.ok) << lowered.refusal.dump();
    EXPECT_NE(std::string::npos, lowered.document.find("SceStatic.door("))
        << "the door of job.completed is not installed:\n"
        << lowered.document;
    const auto machine = std::make_shared<StateMachine>(*engine_);
    const auto raiser = wire(*machine);
    ASSERT_TRUE(machine->loadSCXMLFromString(lowered.document)) << "the Interpreter does not load the document";
    ASSERT_TRUE(machine->start());
    machine->processEvent("job.completed", R"({"elapsed_ms":"soon"})");
    EXPECT_TRUE(settles(raiser, [&] { return machine->getCurrentState() == "refused"; }))
        << "a delivery outside its schema was not refused whole; the machine stands in " << machine->getCurrentState();
}

// A composed payload is judged whole at the door too (docs/adr/0086, section 4.2):
// `static_composed_payload` takes `batch.sent`, whose schema holds lists and records,
// and a delivery whose `items` is a text is refused before any transition is
// selected. The Interpreter answers it with the error.execution taken in its place,
// so the transition it reports is error.execution's, and none on `batch.sent` is
// taken: `item_count` stays 0 and `refusals` counts the error once. A door that let
// the delivery in would report `batch.sent` and leave the refusal to its content.
TEST_F(AStaticDatamodelRunsLoweredUnderTheInterpreterTest, AComposedDeliveryOutsideItsTypeTakesNoTransition) {
    const Lowered lowered = lower(kFixtures / "static_composed_payload.scxml");
    ASSERT_TRUE(lowered.ok) << lowered.refusal.dump();
    const auto machine = std::make_shared<StateMachine>(*engine_);
    const auto raiser = wire(*machine);
    ASSERT_TRUE(machine->loadSCXMLFromString(lowered.document)) << "the Interpreter does not load the document";
    ASSERT_TRUE(machine->start());
    const auto result = machine->processEvent("batch.sent", R"({"items":"none"})");
    EXPECT_EQ("error.execution", result.eventName)
        << "the delivery was taken in place of the error.execution its door owed";
    EXPECT_TRUE(settles(raiser, [&] { return variable(*machine, "refusals") == 1; }))
        << "refusals is " << variable(*machine, "refusals").dump();
    EXPECT_EQ(0, variable(*machine, "item_count"));
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

// A child that declares `<sce:action>`s is given the host its parent's host answers
// for it, each time the invocation starts (`static_child_host.scxml`; the generated
// backends' halves are `a_child_is_given_its_host_by_its_parent`, docs/adr/0005,
// decision 6). The parent declares no act of its own: its host is there for the child
// alone. The first act is `started`, performed as the child is entered, so the host was
// there before the child ran; a state invoked again is a child of its own and is given a
// host of its own, and what the first one did is not carried into the second.
TEST_F(AStaticDatamodelRunsLoweredUnderTheInterpreterTest, AChildIsGivenTheHostItsParentsHostAnswers) {
    const Lowered lowered = lower(kFixtures / "static_child_host.scxml");
    ASSERT_TRUE(lowered.ok) << "a child that declares `<sce:action>`s is lowered for the Interpreter now: "
                            << lowered.refusal.dump();

    const auto parentHost = std::make_shared<ParentHost>();
    const auto machine = std::make_shared<StateMachine>(*engine_);
    // The hosts it answers are recorders, which simulate: accepted by name, and the
    // acceptance covers the children the machine starts (docs/adr/0038).
    machine->setSimulatedNativeActionHost(parentHost);
    const auto raiser = wire(*machine);
    ASSERT_TRUE(machine->loadSCXMLFromString(lowered.document)) << "the Interpreter does not load the document";
    ASSERT_TRUE(machine->start());
    ASSERT_TRUE(settles(raiser, [&] { return !parentHost->answered().empty(); }))
        << "the parent's host was not asked for the child's";

    // Asked once, for `worker`, whose content came inline and so names no document.
    const std::vector<std::pair<std::string, std::string>> firstAsk = {{"worker", ""}};
    ASSERT_EQ(firstAsk, parentHost->asked()) << "the parent's host is asked once for each start";
    const auto first = parentHost->answered().at(0);
    ASSERT_TRUE(settles(raiser, [&] { return first->calls().size() == 1u; }))
        << "the child's first act was not performed by the host it was built with";
    EXPECT_EQ("started", first->calls()[0].first);

    // `a` and `b` are the child's events, which the parent forwards; the second ends it, and
    // it reports the steps it counted to the host it was given.
    machine->processEvent("a", "");
    machine->processEvent("b", "");
    ASSERT_TRUE(settles(raiser, [&] { return variable(*machine, "completed") == 1; }))
        << "the child did not end on `b`";
    const auto ended = first->calls();
    ASSERT_EQ(2u, ended.size());
    EXPECT_EQ("finished", ended[1].first);
    EXPECT_EQ(nlohmann::json::array({2}), ended[1].second);

    // Leaving `working` and coming back invokes `worker` anew: asked again, answered with a
    // host that has done nothing yet, while the first one keeps what it did.
    machine->processEvent("again", "");
    raiser->processQueuedEvents();
    EXPECT_EQ("idle", machine->getCurrentState());
    machine->processEvent("back", "");
    ASSERT_TRUE(settles(raiser, [&] { return parentHost->asked().size() == 2u; }))
        << "the parent's host was not asked again for a child started again";
    const auto answers = parentHost->answered();
    ASSERT_EQ(2u, answers.size());
    EXPECT_NE(answers[0], answers[1]) << "each start is a child of its own and takes a host of its own";
    ASSERT_TRUE(settles(raiser, [&] { return answers[1]->calls().size() == 1u; }));
    EXPECT_EQ("started", answers[1]->calls()[0].first);
    EXPECT_EQ(2u, answers[0]->calls().size()) << "what the first child did is not carried into the second";
}

// A hybrid `<invoke>` gives the candidate it starts the host the parent's host answers
// for THAT document, named by its stem, and for no other (`static_child_host_hybrid.scxml`;
// the generated backends' halves are `a_hybrid_candidate_is_given_its_host_by_its_parent`).
// The first start names `static_hosted_first`, which performs `first_ran`; once `again` has
// changed the value and `back` has entered `working` anew, the second names
// `static_hosted_second`, which performs `second_ran` — an act the first does not declare.
TEST_F(AStaticDatamodelRunsLoweredUnderTheInterpreterTest, ACandidateIsGivenTheHostItsParentsHostAnswersForIt) {
    const auto dir = std::filesystem::temp_directory_path() / ("sce-child-host-hybrid-" + std::to_string(::getpid()));
    std::error_code ec;
    std::filesystem::remove_all(dir, ec);
    const LoweredSet set = lowerSet(kFixtures / "static_child_host_hybrid.scxml", dir);
    ASSERT_TRUE(set.ok) << "a candidate that declares `<sce:action>`s is lowered for the Interpreter now: "
                        << set.refusal.dump();
    ASSERT_EQ(3u, set.documents.size()) << "the invoking document and its two candidates";

    const auto parentHost = std::make_shared<ParentHost>();
    const auto machine = std::make_shared<StateMachine>(*engine_);
    // The hosts it answers are recorders, which simulate: accepted by name, and the
    // acceptance covers the children the machine starts (docs/adr/0038).
    machine->setSimulatedNativeActionHost(parentHost);
    const auto raiser = wire(*machine);
    ASSERT_TRUE(machine->loadSCXML(set.documents[0].string())) << "the Interpreter does not load the document";
    ASSERT_TRUE(machine->start());

    ASSERT_TRUE(settles(raiser, [&] { return variable(*machine, "completed") == 1; }))
        << "the first candidate did not end";
    const std::vector<std::pair<std::string, std::string>> firstAsk = {{"work", "static_hosted_first"}};
    ASSERT_EQ(firstAsk, parentHost->asked());
    const auto first = parentHost->answered().at(0);
    ASSERT_EQ(1u, first->calls().size());
    EXPECT_EQ("first_ran", first->calls()[0].first);

    machine->processEvent("again", "");
    raiser->processQueuedEvents();
    ASSERT_EQ("idle", machine->getCurrentState());
    machine->processEvent("back", "");
    ASSERT_TRUE(settles(raiser, [&] { return variable(*machine, "completed") == 2; }))
        << "the second candidate did not end";
    const std::vector<std::pair<std::string, std::string>> bothAsks = {{"work", "static_hosted_first"},
                                                                       {"work", "static_hosted_second"}};
    ASSERT_EQ(bothAsks, parentHost->asked()) << "the second start asks for the document `pick` names now";
    const auto second = parentHost->answered().at(1);
    ASSERT_EQ(1u, second->calls().size());
    EXPECT_EQ("second_ran", second->calls()[0].first);
    EXPECT_EQ(1u, first->calls().size()) << "the first candidate's host was not asked to do the second's act";
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

// The library's `field` reads a value of any bounded type at its path, or refuses it
// there before anything of it is kept (docs/adr/0086, sections 4 and 14.3): every element
// kind, at list depths 1 to 3, each bound 2, with an empty value, a maximal one, one past
// the bound at the innermost and the outermost list, a bad element, `null` where the
// element is not optional, an unknown key and an older revision, and a top-level text
// held to its bound. The cases are a program the engine runs, whose oracle is the
// construction: each case is built with the value, or the path and the sentence, it must
// come to. The program answers how many cases it asked and which disagreed.
static const char kComposedPayloadCases[] = R"JS((function () {
  var B = 2;
  var CONTROLS = '\u0001\u0001\u0001';
  var REC = {record: {a: 'uint8', b: {string: 3}, c: 'uint8'}, defaults: {c: 9}};
  var REC_LIST = {record: {tags: {list: {string: 3}, max: 2}}};
  var PAST = 'holds 3 elements, past the 2 its type declares';
  var kinds = [
    {name: 'uint8', type: 'uint8', max: 255, bad: 256000000000, below: '', sentence: 'does not fit uint8'},
    {name: 'uint16', type: 'uint16', max: 65535, bad: 256000000000, below: '', sentence: 'does not fit uint16'},
    {name: 'uint32', type: 'uint32', max: 4294967295, bad: 256000000000, below: '', sentence: 'does not fit uint32'},
    {name: 'uint64', type: 'uint64', max: '18446744073709551615', want: BigInt('18446744073709551615'), bad: '-1', below: '', sentence: 'is not a whole number written as digits'},
    {name: 'int8', type: 'int8', max: -128, bad: 1.5, below: '', sentence: 'is not an integer a Number holds exactly'},
    {name: 'int16', type: 'int16', max: -32768, bad: 1.5, below: '', sentence: 'is not an integer a Number holds exactly'},
    {name: 'int32', type: 'int32', max: -2147483648, bad: 1.5, below: '', sentence: 'is not an integer a Number holds exactly'},
    {name: 'int64', type: 'int64', max: '-9223372036854775808', want: BigInt('-9223372036854775808'), bad: 9007199254740992, below: '', sentence: 'is not an integer a Number holds exactly'},
    {name: 'float32', type: 'float32', max: -1.1754943508222875e-38, bad: 1e39, below: '', sentence: 'does not fit float32'},
    {name: 'float64', type: 'float64', max: -2.2250738585072014e-308, bad: true, below: '', sentence: 'is not a number'},
    {name: 'bool', type: 'bool', max: false, bad: 0, below: '', sentence: 'is not a truth value'},
    {name: 'enum', type: ['low', 'medium', 'high'], max: 'medium', bad: 'lowest', below: '', sentence: 'is not a variant of its enum'},
    {name: 'string[3]', type: {string: 3}, max: CONTROLS, bad: 'abcd', below: '', sentence: 'is 4 bytes, past the 3 its type declares'},
    {name: 'bytes[3]', type: {bytes: 3}, max: CONTROLS, bad: 'Ā', below: '', sentence: 'carries a character above U+00FF'},
    {name: 'optional<uint8>', type: {optional: 'uint8'}, max: null, bad: 256000000000, below: '', sentence: 'does not fit uint8'},
    {name: 'record', type: REC, max: {a: 255, b: CONTROLS, c: 255}, bad: {a: 'x', b: ''}, below: '/a', sentence: 'is not a number'},
    {name: 'record of a list', type: REC_LIST, max: {tags: [CONTROLS, CONTROLS]}, bad: {tags: ['', '', 'x']}, below: '/tags', sentence: PAST}
  ];
  function nest(type, depth) {
    for (var i = 0; i < depth; i++) {
      type = {list: type, max: B};
    }
    return type;
  }
  function full(leaf, depth) {
    if (depth === 0) {
      return leaf;
    }
    var items = [];
    for (var i = 0; i < B; i++) {
      items.push(full(leaf, depth - 1));
    }
    return items;
  }
  function withLast(value, depth, leaf) {
    if (depth === 0) {
      return leaf;
    }
    var items = value.slice();
    items[items.length - 1] = withLast(items[items.length - 1], depth - 1, leaf);
    return items;
  }
  function onePast(value, depth, level) {
    var items = value.slice();
    if (level === 1) {
      items.push(items[0]);
    } else {
      items[items.length - 1] = onePast(items[items.length - 1], depth - 1, level - 1);
    }
    return items;
  }
  function innermost(depth) {
    var path = 'v';
    for (var i = 1; i < depth; i++) {
      path += '/1';
    }
    return path;
  }
  function same(a, b) {
    if (Array.isArray(a) || Array.isArray(b)) {
      if (!Array.isArray(a) || !Array.isArray(b) || a.length !== b.length) {
        return false;
      }
      for (var i = 0; i < a.length; i++) {
        if (!same(a[i], b[i])) {
          return false;
        }
      }
      return true;
    }
    if (a !== null && b !== null && typeof a === 'object' && typeof b === 'object') {
      var ka = Object.keys(a).sort();
      var kb = Object.keys(b).sort();
      if (ka.join(',') !== kb.join(',')) {
        return false;
      }
      for (var j = 0; j < ka.length; j++) {
        if (!same(a[ka[j]], b[ka[j]])) {
          return false;
        }
      }
      return true;
    }
    return typeof a === typeof b && String(a) === String(b);
  }
  var cases = 0;
  var failures = [];
  function expect(label, run, want, path, sentence) {
    cases++;
    try {
      var got = run();
      if (sentence !== undefined || !same(got, want)) {
        failures.push(label + ': read ' + String(got));
      }
    } catch (e) {
      var message = String(e && e.message);
      if (sentence === undefined || message.indexOf('\'' + path + '\'') < 0 || message.indexOf(sentence) < 0) {
        failures.push(label + ': ' + message);
      }
    }
  }
  function read(type, value) {
    return SceStatic.field({v: value, zz: {unknown: [1, null]}}, 'v', type);
  }
  kinds.forEach(function (kind) {
    var want = kind.want === undefined ? kind.max : kind.want;
    for (var depth = 1; depth <= 3; depth++) {
      var type = nest(kind.type, depth);
      var max = full(kind.max, depth);
      var leaf = innermost(depth) + '/1';
      var label = kind.name + ' at depth ' + depth + ', ';
      expect(label + 'empty', function () { return read(type, []); }, []);
      expect(label + 'maximal', function () { return read(type, max); }, full(want, depth));
      expect(label + 'one past the innermost bound', function () { return read(type, onePast(max, depth, depth)); }, undefined, innermost(depth), PAST);
      expect(label + 'one past the outermost bound', function () { return read(type, onePast(max, depth, 1)); }, undefined, 'v', PAST);
      if (kind.name === 'optional<uint8>') {
        expect(label + 'a present element among absent ones', function () { return read(type, withLast(max, depth, 7)); }, withLast(full(want, depth), depth, 7));
      } else {
        expect(label + 'null where the element is not optional', function () { return read(type, withLast(max, depth, null)); }, undefined, leaf, 'is null, and its type is not optional');
      }
      expect(label + 'a bad element', function () { return read(type, withLast(max, depth, kind.bad)); }, undefined, leaf + kind.below, kind.sentence);
      if (kind.name === 'record') {
        expect(label + 'an unknown key', function () { return read(type, withLast(max, depth, {zz: [{q: 1}], c: 255, b: CONTROLS, a: 255})); }, full(want, depth));
        expect(label + 'an element of an older revision', function () { return read(type, withLast(max, depth, {a: 1, b: ''})); }, withLast(full(want, depth), depth, {a: 1, b: '', c: 9}));
      } else {
        expect(label + 'an unknown key', function () { return read(type, max); }, full(want, depth));
        expect(label + 'a delivery of the revision before the list', function () { return SceStatic.field({zz: 1}, 'v', type, []); }, []);
      }
    }
  });
  expect('a top-level text past its bound', function () { return SceStatic.field({note: 'abcde'}, 'note', {string: 4}); }, undefined, 'note', 'is 5 bytes, past the 4 its type declares');
  expect('a list past its bound before its first element is read', function () { return SceStatic.field({v: ['x', 1, 2]}, 'v', {list: 'uint8', max: 2}); }, undefined, 'v', PAST);
  expect('a record that lacks a field', function () { return SceStatic.field({r: {a: 1}}, 'r', {record: {a: 'uint8', b: 'uint8'}}); }, undefined, 'r/b', 'the event data has no');
  expect('a number where a record stands', function () { return SceStatic.field({s: 5}, 's', REC); }, undefined, 's', 'is not a record');
  return JSON.stringify({cases: cases, failures: failures});
})())JS";

TEST_F(AStaticDatamodelRunsLoweredUnderTheInterpreterTest, TheLibraryReadsAValueOfAnyBoundedTypeAtItsPath) {
    const char *bin = std::getenv("SCE_CODEGEN_BIN");
    ASSERT_NE(bin, nullptr) << "SCE_CODEGEN_BIN must be set by CMake add_test ENVIRONMENT";
    // Any document that is lowered installs the library; an algorithm is the
    // smallest, and `lower-algorithm` hands over the expression that installs it.
    const Ran ran =
        capture(std::string("\"") + bin + "\" lower-algorithm sce:std/time/second_of_day.scxml 2>/dev/null");
    ASSERT_EQ(0, ran.status);
    const auto line = nlohmann::json::parse(ran.output, nullptr, false);
    ASSERT_TRUE(line.is_object()) << ran.output.substr(0, 200);

    const std::string session = "composed_payload_lowered";
    ASSERT_TRUE(engine_->createSession(session, ""));
    const auto installed = engine_->executeScript(session, line.at("install").get<std::string>()).get();
    ASSERT_TRUE(installed.isSuccess()) << installed.getErrorMessage();

    const auto answered = engine_->evaluateExpression(session, kComposedPayloadCases).get();
    ASSERT_TRUE(answered.isSuccess()) << answered.getErrorMessage();
    const auto report = nlohmann::json::parse(answered.getValue<std::string>(), nullptr, false);
    ASSERT_TRUE(report.is_object()) << answered.getValue<std::string>().substr(0, 200);
    EXPECT_EQ(report.at("cases").get<int>(), 17 * 3 * 8 + 4) << "the cross product shrank";
    EXPECT_TRUE(report.at("failures").empty()) << report.at("failures").dump(2);
    engine_->destroySession(session);
}

// The library's refusal carries, beside its sentence, the code, the RFC 6901 pointer and
// the bound every runtime's reader names (docs/adr/0087, sections 2.3 and 2.4), held to
// tests/json_text/payload_refusal.json, the table every reader reads. The text of each
// case is read by the library's own reader of JSON text (`SceStatic.parseText`), which
// holds it to RFC 8259 alone as every runtime's reader does, a repeated name and a lax
// number included, so every case of the table is the library's; a case that gives a
// depth is read as a `<sce:lift>` reads its text (`SceStatic.lift`).
TEST_F(AStaticDatamodelRunsLoweredUnderTheInterpreterTest, TheLibraryNamesEachRefusalByItsCodePointerAndBound) {
    const char *bin = std::getenv("SCE_CODEGEN_BIN");
    ASSERT_NE(bin, nullptr) << "SCE_CODEGEN_BIN must be set by CMake add_test ENVIRONMENT";
    const Ran ran =
        capture(std::string("\"") + bin + "\" lower-algorithm sce:std/time/second_of_day.scxml 2>/dev/null");
    ASSERT_EQ(0, ran.status);
    const auto line = nlohmann::json::parse(ran.output, nullptr, false);
    ASSERT_TRUE(line.is_object()) << ran.output.substr(0, 200);

    const std::string session = "payload_refusal_lowered";
    ASSERT_TRUE(engine_->createSession(session, ""));
    const auto installed = engine_->executeScript(session, line.at("install").get<std::string>()).get();
    ASSERT_TRUE(installed.isSuccess()) << installed.getErrorMessage();

    const auto table =
        nlohmann::json::parse(slurp(std::filesystem::path(SCE_PROJECT_ROOT) / "tests/json_text/payload_refusal.json"));
    const auto &cases = table.at("cases");
    ASSERT_GE(cases.size(), 40u) << "the shared table was not read";
    size_t asked = 0;
    for (const auto &c : cases) {
        const std::string name = c.at("name").get<std::string>();
        const std::string data = c.at("data").get<std::string>();
        ++asked;
        // A delivery's text that is no JSON is refused with no data read; a lift's text is
        // read whole by `SceStatic.lift`, the record of the one field `v` its type, its
        // pointer held to `pathBound` bytes, the standard refusal's 256 where none is given.
        const std::string reading =
            c.contains("depth") ? "var lifted = SceStatic.lift(text, {record: {'v': " + c.at("type").dump() + "}}, " +
                                      c.at("depth").dump() + ", " + std::to_string(c.value("pathBound", 256)) +
                                      "); if (lifted.refusal !== null) { return JSON.stringify({refusal: "
                                      "lifted.refusal}); }"
                                : "var data; if (text !== '') { try { data = SceStatic.parseText(text); } catch (e) { "
                                  "return JSON.stringify({refusal: {code: 'notJson', pointer: '', bound: null}}); } } "
                                  "SceStatic.field(data, 'v', " +
                                      c.at("type").dump() + ");";
        const std::string program = "(function () { var text = " + nlohmann::json(data).dump() + "; try { " + reading +
                                    " return JSON.stringify({read: true}); } catch (e) { return "
                                    "JSON.stringify({refusal: e.sceRefusal === undefined ? null : e.sceRefusal, "
                                    "message: String(e.message)}); } })()";
        const auto answered = engine_->evaluateExpression(session, program).get();
        ASSERT_TRUE(answered.isSuccess()) << name << ": " << answered.getErrorMessage();
        const auto got = nlohmann::json::parse(answered.getValue<std::string>(), nullptr, false);
        ASSERT_TRUE(got.is_object()) << name << ": " << answered.getValue<std::string>();
        if (!c.contains("refusal")) {
            EXPECT_TRUE(got.contains("read")) << name << ": " << got.dump();
            continue;
        }
        ASSERT_TRUE(got.contains("refusal") && got.at("refusal").is_object()) << name << ": " << got.dump();
        const auto &want = c.at("refusal");
        const auto &refusal = got.at("refusal");
        EXPECT_EQ(refusal.at("code"), want.at("code")) << name << ": " << got.dump();
        EXPECT_EQ(refusal.at("pointer"), want.at("pointer")) << name << ": " << got.dump();
        EXPECT_EQ(refusal.at("bound"), want.at("bound")) << name << ": " << got.dump();
    }
    EXPECT_EQ(asked, cases.size()) << "every case of the table is the library's";
    engine_->destroySession(session);
}

// A union is read tag first and then the record of the case its tag names, from the same
// object, and held as the wire's form: the case's fields beside its tag, a key or a `null`
// the case does not declare left behind (docs/adr/0092, sections 4 and 7). Two cases that
// hold one record stay apart by their tag.
TEST_F(AStaticDatamodelRunsLoweredUnderTheInterpreterTest, TheLibraryReadsAUnionAsTheCaseItsTagNames) {
    const char *bin = std::getenv("SCE_CODEGEN_BIN");
    ASSERT_NE(bin, nullptr) << "SCE_CODEGEN_BIN must be set by CMake add_test ENVIRONMENT";
    const Ran ran =
        capture(std::string("\"") + bin + "\" lower-algorithm sce:std/time/second_of_day.scxml 2>/dev/null");
    ASSERT_EQ(0, ran.status);
    const auto line = nlohmann::json::parse(ran.output, nullptr, false);
    ASSERT_TRUE(line.is_object()) << ran.output.substr(0, 200);

    const std::string session = "union_payload_lowered";
    ASSERT_TRUE(engine_->createSession(session, ""));
    const auto installed = engine_->executeScript(session, line.at("install").get<std::string>()).get();
    ASSERT_TRUE(installed.isSuccess()) << installed.getErrorMessage();

    const std::string shape = "{union: {tag: 'kind', cases: {dot: null, circle: {record: {radius: 'uint16'}}, disc: "
                              "{record: {radius: 'uint16'}}}}}";

    const struct {
        const char *data;
        const char *held;
    } cases[] = {
        {R"({"s":{"kind":"dot"}})", R"({"kind":"dot"})"},
        {R"({"s":{"radius":2,"kind":"disc"}})", R"({"kind":"disc","radius":2})"},
        {R"({"s":{"kind":"circle","radius":2,"width":null,"text":"x"}})", R"({"kind":"circle","radius":2})"},
        {R"({"s":{"kind":"dot","radius":null}})", R"({"kind":"dot"})"},
    };

    for (const auto &c : cases) {
        const std::string program = std::string("(function () { var s = SceStatic.field(SceStatic.parseText(") +
                                    nlohmann::json(c.data).dump() + "), 's', " + shape +
                                    "); var keys = Object.keys(s).sort(); var out = {}; keys.forEach(function (k) { "
                                    "out[k] = s[k]; }); return JSON.stringify(out); })()";
        const auto answered = engine_->evaluateExpression(session, program).get();
        ASSERT_TRUE(answered.isSuccess()) << c.data << ": " << answered.getErrorMessage();
        EXPECT_EQ(answered.getValue<std::string>(), c.held) << c.data;
    }
    engine_->destroySession(session);
}

// A lift raises nothing (docs/adr/0087, section 2.1), and this engine's data model holds
// an integer only up to 2^53 - 1: a 64-bit integer past that, which a generated backend's
// int64 holds and its lift reads, is refused here as a whole number past the width the
// engine gives the type (`outOfRange`, at the field), and never thrown as unrepresentable.
// The answer is this engine's own, so it is pinned here and not in the shared table.
TEST_F(AStaticDatamodelRunsLoweredUnderTheInterpreterTest, ALiftOfAnIntegerTheDataModelCannotHoldIsRefusedNotThrown) {
    const char *bin = std::getenv("SCE_CODEGEN_BIN");
    ASSERT_NE(bin, nullptr) << "SCE_CODEGEN_BIN must be set by CMake add_test ENVIRONMENT";
    const Ran ran =
        capture(std::string("\"") + bin + "\" lower-algorithm sce:std/time/second_of_day.scxml 2>/dev/null");
    ASSERT_EQ(0, ran.status);
    const auto line = nlohmann::json::parse(ran.output, nullptr, false);
    ASSERT_TRUE(line.is_object()) << ran.output.substr(0, 200);

    const std::string session = "lift_unheld_integer";
    ASSERT_TRUE(engine_->createSession(session, ""));
    const auto installed = engine_->executeScript(session, line.at("install").get<std::string>()).get();
    ASSERT_TRUE(installed.isSuccess()) << installed.getErrorMessage();

    struct Case {
        const char *text;
        const char *type;  // the type of the record's one field `v`, as the library writes it
        const char *code;  // nullptr: read, `v` the value `read`
        const char *read;
    };

    const Case cases[] = {
        {R"({"v":"9007199254740991"})", "'int64'", nullptr, "9007199254740991"},
        {R"({"v":"-9007199254740991"})", "'int64'", nullptr, "-9007199254740991"},
        {R"({"v":"9007199254740992"})", "'int64'", "outOfRange", nullptr},
        {R"({"v":"9007199254740993"})", "'int64'", "outOfRange", nullptr},
        {R"({"v":"9223372036854775807"})", "'int64'", "outOfRange", nullptr},
        {R"({"v":"-9223372036854775808"})", "'int64'", "outOfRange", nullptr},
        {R"({"v":"18446744073709551615"})", "'uint64'", "outOfRange", nullptr},
        {R"({"v":"9223372036854775807"})", "{optional: 'int64'}", "outOfRange", nullptr},
        {R"({"v":9007199254740993})", "'int64'", "badNumber", nullptr},
    };
    for (const Case &c : cases) {
        const std::string program = std::string("(function () { var text = ") + nlohmann::json(c.text).dump() +
                                    "; try { var lifted = SceStatic.lift(text, {record: {'v': " + c.type +
                                    "}}, 1, 256); return JSON.stringify(lifted.refusal !== null ? {refusal: "
                                    "lifted.refusal} : {read: String(lifted.value.v)}); } catch (e) { return "
                                    "JSON.stringify({threw: String(e.message)}); } })()";
        const auto answered = engine_->evaluateExpression(session, program).get();
        ASSERT_TRUE(answered.isSuccess()) << c.text << ": " << answered.getErrorMessage();
        const auto got = nlohmann::json::parse(answered.getValue<std::string>(), nullptr, false);
        ASSERT_TRUE(got.is_object()) << c.text << ": " << answered.getValue<std::string>();
        EXPECT_FALSE(got.contains("threw")) << c.text << ": the lift threw: " << got.dump();
        if (c.code == nullptr) {
            EXPECT_EQ(got.value("read", std::string()), c.read) << c.text << ": " << got.dump();
            continue;
        }
        ASSERT_TRUE(got.contains("refusal")) << c.text << ": " << got.dump();
        EXPECT_EQ(got.at("refusal").at("code"), c.code) << c.text << ": " << got.dump();
        EXPECT_EQ(got.at("refusal").at("pointer"), "/v") << c.text << ": " << got.dump();
        EXPECT_TRUE(got.at("refusal").at("bound").is_null()) << c.text << ": " << got.dump();
    }
    engine_->destroySession(session);
}

// The library's `payload` reads the whole data of an event whose schema holds a composed
// value (docs/adr/0086, section 6): every field as `field` reads it, a field a later
// revision added as its default where the delivery lacks it, or the refusal of the first
// place that is not one, with its code, pointer and bound. An integer past 2^53 - 1, which
// a part walked into the data model would lose, is refused at its pointer as a lift
// refuses it (`outOfRange`), and never handed on.
TEST_F(AStaticDatamodelRunsLoweredUnderTheInterpreterTest, TheLibraryReadsAComposedPayloadWholeOrRefusesItWhole) {
    const char *bin = std::getenv("SCE_CODEGEN_BIN");
    ASSERT_NE(bin, nullptr) << "SCE_CODEGEN_BIN must be set by CMake add_test ENVIRONMENT";
    const Ran ran =
        capture(std::string("\"") + bin + "\" lower-algorithm sce:std/time/second_of_day.scxml 2>/dev/null");
    ASSERT_EQ(0, ran.status);
    const auto line = nlohmann::json::parse(ran.output, nullptr, false);
    ASSERT_TRUE(line.is_object()) << ran.output.substr(0, 200);

    const std::string session = "composed_payload_whole";
    ASSERT_TRUE(engine_->createSession(session, ""));
    const auto installed = engine_->executeScript(session, line.at("install").get<std::string>()).get();
    ASSERT_TRUE(installed.isSuccess()) << installed.getErrorMessage();

    const std::string type = "{record: {'xs': {list: 'uint8', max: 2}, 'n': 'int64', 'late': {list: 'uint8', max: "
                             "2}}, defaults: {\"late\": []}}";

    struct Case {
        const char *data;
        const char *read;     // nullptr: refused
        const char *code;     // the refusal's code
        const char *pointer;  // the refusal's pointer
        int bound;            // the refusal's bound, -1 for none
    };

    const Case cases[] = {
        {R"({"xs":[1,2],"n":"5"})", R"({"xs":[1,2],"n":5,"late":[]})", nullptr, nullptr, -1},
        {R"({"late":[3],"n":"5","xs":[]})", R"({"xs":[],"n":5,"late":[3]})", nullptr, nullptr, -1},
        {R"({"xs":[1,2,3],"n":"5"})", nullptr, "tooMany", "/xs", 2},
        {R"({"xs":[],"n":"5","late":[1,2,3]})", nullptr, "tooMany", "/late", 2},
        {R"({"n":"5"})", nullptr, "missing", "/xs", -1},
        {R"({"xs":[1],"n":"9007199254740993"})", nullptr, "outOfRange", "/n", -1},
    };
    for (const Case &c : cases) {
        const std::string program = std::string("(function () { try { return JSON.stringify({read: "
                                                "SceStatic.payload(SceStatic.parseText(") +
                                    nlohmann::json(c.data).dump() + "), " + type +
                                    ")}); } catch (e) { return JSON.stringify({refusal: e.sceRefusal === undefined ? "
                                    "null : e.sceRefusal, message: String(e.message)}); } })()";
        const auto answered = engine_->evaluateExpression(session, program).get();
        ASSERT_TRUE(answered.isSuccess()) << c.data << ": " << answered.getErrorMessage();
        const auto got = nlohmann::json::parse(answered.getValue<std::string>(), nullptr, false);
        ASSERT_TRUE(got.is_object()) << c.data << ": " << answered.getValue<std::string>();
        if (c.read != nullptr) {
            ASSERT_TRUE(got.contains("read")) << c.data << ": " << got.dump();
            // Compared as JSON: the order of the members is no part of the answer.
            EXPECT_EQ(got.at("read"), nlohmann::json::parse(c.read)) << c.data;
            continue;
        }
        ASSERT_TRUE(got.contains("refusal") && got.at("refusal").is_object()) << c.data << ": " << got.dump();
        const auto &refusal = got.at("refusal");
        EXPECT_EQ(refusal.at("code"), c.code) << c.data << ": " << got.dump();
        EXPECT_EQ(refusal.at("pointer"), c.pointer) << c.data << ": " << got.dump();
        if (c.bound < 0) {
            EXPECT_TRUE(refusal.at("bound").is_null()) << c.data << ": " << got.dump();
        } else {
            EXPECT_EQ(refusal.at("bound"), c.bound) << c.data << ": " << got.dump();
        }
    }
    engine_->destroySession(session);
}

namespace {

/// One item of `batch.sent` as JSON (`event_batch_item.scxml`): `tags` holds that many
/// texts, and `amount` is the largest integer the Interpreter's data model holds exactly.
std::string composedItem(int id, int tags) {
    std::string listed;
    for (int n = 0; n < tags; ++n) {
        listed += (n > 0 ? "," : "") + std::string("\"tag") + std::to_string(n) + "\"";
    }
    return "{\"id\":" + std::to_string(id) +
           ",\"amount\":\"9007199254740991\",\"label\":\"abcdefgh\",\"level\":\"high\",\"due\":null,"
           "\"blob\":\"wxyz\",\"tags\":[" +
           listed + "]}";
}

/// A delivery of `batch.sent` (`event_batch.scxml`) whose every list is full, the second
/// item's `tags` holding `secondTags` texts.
std::string composedBatch(int secondTags) {
    return "{\"items\":[" + composedItem(1, 4) + "," + composedItem(2, secondTags) + "," + composedItem(3, 4) +
           "],\"grid\":[[1,2],[3,4]],\"levels\":[\"low\",\"critical\"],\"maybe\":[7,null],\"spare\":[1,2],"
           "\"note\":\"abcd\",\"late\":[5,6]}";
}

/// `text` with its first `from` written `to`.
std::string withFirst(std::string text, const std::string &from, const std::string &to) {
    const auto at = text.find(from);
    EXPECT_NE(at, std::string::npos) << "`" << from << "` is not in " << text;
    if (at != std::string::npos) {
        text.replace(at, from.size(), to);
    }
    return text;
}

}  // namespace

// The Interpreter reads a composed payload at any depth (docs/adr/0086, section 6), as the
// generated machines do (AComposedPayloadIsReadAtAnyDepthTest and its siblings): the
// lowered `static_composed_payload` measures a list of the payload, walks it with an index
// and each item's `tags`, reads a field of an element through its index, walks a list of
// lists, reads `grid[1][0]`, an element that may be absent and the length of the first
// item's `tags`, and takes `late` whole into a list of a larger bound.
TEST_F(AStaticDatamodelRunsLoweredUnderTheInterpreterTest, AComposedPayloadIsReadAtAnyDepth) {
    const Lowered lowered = lower(kFixtures / "static_composed_payload.scxml");
    ASSERT_TRUE(lowered.ok) << lowered.refusal.dump();
    const auto machine = std::make_shared<StateMachine>(*engine_);
    const auto raiser = wire(*machine);
    ASSERT_TRUE(machine->loadSCXMLFromString(lowered.document)) << "the Interpreter does not load the document";
    ASSERT_TRUE(machine->start());
    machine->processEvent("batch.sent", composedBatch(4));
    ASSERT_TRUE(settles(raiser, [&] { return variable(*machine, "item_count") == 3; }))
        << "the length of `items` was not read: refusals " << variable(*machine, "refusals").dump();
    EXPECT_EQ(variable(*machine, "refusals"), 0) << "a delivery of its type is no error";
    // Depth 2: a loop over each item's list, and a field of an element read through its index.
    EXPECT_EQ(variable(*machine, "tag_count"), 12);
    EXPECT_EQ(variable(*machine, "word_count"), 12);
    EXPECT_EQ(variable(*machine, "id_sum"), 6);
    // A list of lists, walked at both depths and read by two indices.
    EXPECT_EQ(variable(*machine, "cell_sum"), 10);
    EXPECT_EQ(variable(*machine, "cell"), 3);
    // Depth 3: the length of the first item's `tags`.
    EXPECT_EQ(variable(*machine, "first_tags"), 4);
    // An element that may be absent, present.
    EXPECT_EQ(variable(*machine, "first_maybe"), 7);
    // A list taken whole into a list of a larger bound.
    EXPECT_EQ(variable(*machine, "late"), nlohmann::json::array({5, 6}));
}

// A delivery outside its type is refused whole by the Interpreter too (docs/adr/0086,
// sections 4.2 and 5.4): a list one past its bound at depth two, a text past its bound at
// depth three and at the top level, a list a later revision added one past its bound, and
// an integer past what the data model holds. Each is refused by the read of the whole data
// the lowering opens the transition with: none of the content runs, nothing of the delivery
// is kept, and error.execution reaches the document once for each.
TEST_F(AStaticDatamodelRunsLoweredUnderTheInterpreterTest, AComposedPayloadOutsideItsTypeIsRefusedWhole) {
    const Lowered lowered = lower(kFixtures / "static_composed_payload.scxml");
    ASSERT_TRUE(lowered.ok) << lowered.refusal.dump();
    const auto machine = std::make_shared<StateMachine>(*engine_);
    const auto raiser = wire(*machine);
    ASSERT_TRUE(machine->loadSCXMLFromString(lowered.document)) << "the Interpreter does not load the document";
    ASSERT_TRUE(machine->start());
    const std::string full = composedBatch(4);
    const std::vector<std::pair<std::string, std::string>> deliveries = {
        {"`items/1/tags` holds five texts, past its four", composedBatch(5)},
        {"`items/0/tags/0` is nine bytes, past its eight", withFirst(full, "\"tag0\"", "\"ninebytes\"")},
        {"`note` is five bytes, past its four", withFirst(full, "\"note\":\"abcd\"", "\"note\":\"abcde\"")},
        {"`late` holds three, past its two", withFirst(full, "\"late\":[5,6]", "\"late\":[5,6,7]")},
        {"`items/0/amount` is past 2^53 - 1",
         withFirst(full, "\"amount\":\"9007199254740991\"", "\"amount\":\"9007199254740993\"")},
    };
    int refused = 0;
    for (const auto &[why, data] : deliveries) {
        SCOPED_TRACE(why);
        machine->processEvent("batch.sent", data);
        ++refused;
        EXPECT_TRUE(settles(raiser, [&] { return variable(*machine, "refusals") == refused; }))
            << "refusals is " << variable(*machine, "refusals").dump() << ", not " << refused;
        EXPECT_EQ(variable(*machine, "item_count"), 0) << "none of the content ran";
        EXPECT_EQ(variable(*machine, "late"), nlohmann::json::array()) << "nothing of the delivery was kept";
    }
}

// The Interpreter sends a composed value by its name (docs/adr/0086, section 5.2), as the
// generated machines do (AComposedValueIsSentByItsNameTest and its siblings): the lowered
// `static_composed_send` forwards its first delivery to itself, four lists and an optional
// list named as the parts of the payload they are and `late` as the list variable it took
// the payload's into, and the delivery that comes back is read whole and fits its schema.
TEST_F(AStaticDatamodelRunsLoweredUnderTheInterpreterTest, AComposedValueIsSentByItsName) {
    const Lowered lowered = lower(kFixtures / "static_composed_send.scxml");
    ASSERT_TRUE(lowered.ok) << lowered.refusal.dump();
    const auto machine = std::make_shared<StateMachine>(*engine_);
    const auto raiser = wire(*machine);
    ASSERT_TRUE(machine->loadSCXMLFromString(lowered.document)) << "the Interpreter does not load the document";
    ASSERT_TRUE(machine->start());
    machine->processEvent("batch.sent", composedBatch(3));
    ASSERT_TRUE(settles(raiser, [&] { return variable(*machine, "deliveries") == 2; }))
        << "the forwarded delivery did not come back: deliveries " << variable(*machine, "deliveries").dump()
        << ", refusals " << variable(*machine, "refusals").dump();
    EXPECT_EQ(variable(*machine, "refusals"), 0) << "the delivery the machine sent itself fits its schema";
    // Depth 1: the lengths of a list of records and a list of enum values.
    EXPECT_EQ(variable(*machine, "item_count"), 3);
    EXPECT_EQ(variable(*machine, "level_count"), 2);
    // Depth 2: each item's list, and a field of an element read by its index.
    EXPECT_EQ(variable(*machine, "tag_count"), 11);
    EXPECT_EQ(variable(*machine, "id_sum"), 6);
    // A list of lists, walked at both depths.
    EXPECT_EQ(variable(*machine, "cell_sum"), 10);
    // An element that may be absent, present.
    EXPECT_EQ(variable(*machine, "first_maybe"), 7);
    // The list variable, sent by its name.
    EXPECT_EQ(variable(*machine, "late_sum"), 11);
    EXPECT_EQ(variable(*machine, "late"), nlohmann::json::array({5, 6}));
}

// A delivery outside its type runs none of its transition's content on the Interpreter,
// as on the generated machines, which lift the payload when the event is dequeued
// (docs/adr/0086, section 4.2): `static_composed_send`'s first transition counts the
// delivery before any statement of it reads the data, and the read the lowering opens the
// transition with refuses the delivery first, so the count is not taken. A delivery of
// its type then runs the transition whole.
TEST_F(AStaticDatamodelRunsLoweredUnderTheInterpreterTest, ARefusedComposedDeliveryRunsNoContentOfItsTransition) {
    const Lowered lowered = lower(kFixtures / "static_composed_send.scxml");
    ASSERT_TRUE(lowered.ok) << lowered.refusal.dump();
    const auto machine = std::make_shared<StateMachine>(*engine_);
    const auto raiser = wire(*machine);
    ASSERT_TRUE(machine->loadSCXMLFromString(lowered.document)) << "the Interpreter does not load the document";
    ASSERT_TRUE(machine->start());
    machine->processEvent("batch.sent", composedBatch(5));
    EXPECT_TRUE(settles(raiser, [&] { return variable(*machine, "refusals") == 1; }))
        << "refusals is " << variable(*machine, "refusals").dump();
    EXPECT_EQ(variable(*machine, "deliveries"), 0) << "the <assign> before the first read of the data ran";
    EXPECT_EQ(variable(*machine, "late"), nlohmann::json::array()) << "nothing of the delivery was kept";
    machine->processEvent("batch.sent", composedBatch(3));
    EXPECT_TRUE(settles(raiser, [&] { return variable(*machine, "deliveries") == 2; }))
        << "a delivery of its type after the refusal was not forwarded and read: deliveries "
        << variable(*machine, "deliveries").dump();
    EXPECT_EQ(variable(*machine, "refusals"), 1) << "the delivery of its type is no error";
    EXPECT_EQ(variable(*machine, "item_count"), 3);
}

// SCE_MESH.md, Errors: a send to a peer from a machine whose host registered no Mesh
// router is a configuration fault of the host and raises error.execution, as it does
// on every other backend. It is not an event the sender takes from itself:
// `todo_peer` takes what a peer sends it as a change to its own list, so an event that
// came back would be the peer's, and the change the machine took in over `#_internal`
// is the only one it holds.
TEST_F(AStaticDatamodelRunsLoweredUnderTheInterpreterTest, ASendToAMeshPeerWithNoRouterRegisteredIsErrorExecution) {
    const Lowered lowered = lower(kFixtures / "todo_peer.scxml");
    ASSERT_TRUE(lowered.ok) << lowered.refusal.dump();
    const auto machine = std::make_shared<StateMachine>(*engine_);
    const auto raiser = wire(*machine);
    ASSERT_TRUE(machine->loadSCXMLFromString(lowered.document)) << "the Interpreter does not load the document";
    ASSERT_TRUE(machine->start());
    machine->processEvent("todo.join", R"({"node":"1"})");
    machine->processEvent("todo.edit", R"({"id":7,"field":1,"value":"0","text":"buy milk","now":"0"})");
    EXPECT_TRUE(settles(raiser, [&] { return variable(*machine, "refusals") == 1; }))
        << "the send to #people was not refused as error.execution, once";
}

// SCE_MESH.md, Child Sessions in Mesh (docs/adr/0043): a child session is built with
// a dispatcher of its own, and is given none of its parent's Mesh route. Here the
// machine's host registered a router for the machine and no factory for its
// children, so `list`'s greeting to `#server` is refused as error.execution in the
// child, which ends on it, and the machine counts the end. A child built with its
// parent's dispatcher would have sent the greeting through the parent's router,
// under the parent's name, and the machine would have counted nothing.
TEST_F(AStaticDatamodelRunsLoweredUnderTheInterpreterTest, AChildIsNotGivenItsParentsMeshRoute) {
    const auto dir = std::filesystem::temp_directory_path() / ("sce-child-mesh-" + std::to_string(::getpid()));
    std::error_code ec;
    std::filesystem::remove_all(dir, ec);
    const LoweredSet set = lowerSet(kFixtures / "static_child_mesh.scxml", dir);
    ASSERT_TRUE(set.ok) << set.refusal.dump();

    const auto machine = std::make_shared<StateMachine>(*engine_);
    const auto sentToPeers = std::make_shared<SentToPeers>();
    const auto raiser = wire(*machine, sentToPeers);
    ASSERT_TRUE(machine->loadSCXML(set.documents.at(0).string())) << "the Interpreter does not load the document";
    ASSERT_TRUE(machine->start());
    EXPECT_TRUE(settles(raiser, [&] { return variable(*machine, "refusals") == 1; }))
        << "the child's greeting to #server was not refused in the child";
    EXPECT_EQ(nlohmann::json::array(), sentToPeers->take()) << "the child sent under its parent's name";
    machine->stop();
    std::filesystem::remove_all(dir, ec);
}

// A child that is given a router sends through it, under its own invoke, and the
// router is told once that the session ended when the parent leaves the invoking
// state (SCE_MESH.md, Child Sessions in Mesh).
TEST_F(AStaticDatamodelRunsLoweredUnderTheInterpreterTest, AChildSendsThroughTheRouterItsParentAsksFor) {
    const auto dir = std::filesystem::temp_directory_path() / ("sce-child-mesh-router-" + std::to_string(::getpid()));
    std::error_code ec;
    std::filesystem::remove_all(dir, ec);
    const LoweredSet set = lowerSet(kFixtures / "static_child_mesh.scxml", dir);
    ASSERT_TRUE(set.ok) << set.refusal.dump();

    struct Router : IChildMeshRouter {
        explicit Router(std::shared_ptr<SentToPeers> sent) : sent(std::move(sent)) {}

        std::shared_ptr<IEventTarget> createTarget(const std::string &targetUri) override {
            return std::make_shared<PeerTarget>(sent, targetUri, nlohmann::json{{"invoke", "list"}});
        }

        void attach(std::shared_ptr<IEventRaiser>) override {
            ++attached;
        }

        void detach() override {
            ++detached;
        }

        std::shared_ptr<SentToPeers> sent;
        std::atomic<int> attached{0};
        std::atomic<int> detached{0};
    };

    const auto machine = std::make_shared<StateMachine>(*engine_);
    const auto sentToPeers = std::make_shared<SentToPeers>();
    const auto childSent = std::make_shared<SentToPeers>();
    std::vector<std::pair<std::string, std::string>> asked;
    std::shared_ptr<Router> router;
    machine->registerChildMeshRouters(
        [&](const std::string &invokeId, const std::string &document) -> std::shared_ptr<IChildMeshRouter> {
            asked.emplace_back(invokeId, document);
            if (invokeId != "list" || router) {
                return nullptr;
            }
            router = std::make_shared<Router>(childSent);
            return router;
        });
    const auto raiser = wire(*machine, sentToPeers);
    ASSERT_TRUE(machine->loadSCXML(set.documents.at(0).string())) << "the Interpreter does not load the document";
    ASSERT_TRUE(machine->start());
    ASSERT_TRUE(settles(raiser, [&] { return childSent->peek().size() == 1u; }))
        << "the child's greeting did not reach the router it was given";
    const std::vector<std::pair<std::string, std::string>> firstAsk = {{"list", ""}};
    EXPECT_EQ(firstAsk, asked) << "the factory is asked with the invoke's id and no document for an inline child";
    EXPECT_EQ((nlohmann::json::array({{{"target", "#server"}, {"event", "list.hello"}, {"invoke", "list"}}})),
              childSent->take());
    EXPECT_EQ(nlohmann::json::array(), sentToPeers->take()) << "the child's send reached its parent's router";
    EXPECT_EQ(1, router->attached.load()) << "the router was attached once, before the child started";
    EXPECT_EQ(0, variable(*machine, "refusals")) << "the child given a router had its send refused";

    machine->processEvent("again", "");
    EXPECT_TRUE(settles(raiser, [&] { return router->detached.load() == 1; }))
        << "the router was not told the session ended when its parent left the invoking state";
    machine->stop();
    std::filesystem::remove_all(dir, ec);
}

}  // namespace Tests
}  // namespace SCE
